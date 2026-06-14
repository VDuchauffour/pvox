pub mod app;
pub mod client;
pub mod config;
pub mod event;
pub mod theme;
pub mod tui;
pub mod ui;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::Event;
use futures::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::App;
use crate::client::{ProxmoxClient, TaskStatus};
use crate::config::Config;
use crate::event::{AppEvent, LifecycleAction};
use crate::tui::Tui;

pub async fn run(config: Config) -> Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();

    let mut tui = Tui::new()?;
    let mut app = App::new(config)?;

    if let Some(ref client) = app.client {
        spawn_polling_task(tx.clone(), client.clone(), app.config.refresh_interval);
        spawn_version_task(tx.clone(), client.clone());
        spawn_whoami_task(tx.clone(), client.clone());
    }

    spawn_event_task(tx.clone());
    spawn_tick_task(tx.clone());

    loop {
        tokio::select! {
            biased;
            Some(event) = rx.recv() => {
                match event {
                    AppEvent::Tick => {
                        tui.terminal.draw(|frame| {
                            ui::render(frame, &app);
                        })?;
                    }
                    AppEvent::Key(key) => {
                        app.handle_key(key, &tx);
                        if app.quit {
                            break;
                        }
                    }
                    AppEvent::Resize(_w, _h) => {
                        // resize handling (optional for now)
                    }
                    AppEvent::ClusterSnapshot(resources) => {
                        app.connected = true;
                        app.set_resources(resources);
                    }
                    AppEvent::VersionSnapshot(version) => {
                        app.proxmox_version = version;
                    }
                    AppEvent::WhoAmiSnapshot(user) => {
                        app.proxmox_user = user;
                    }
                    AppEvent::ApiError(err) => {
                        app.connected = false;
                        app.status_message = Some(err);
                    }
                    AppEvent::LifecycleComplete(upid) => {
                        if let Some(done_upid) = upid.strip_prefix("DONE:") {
                            app.complete_upid(done_upid);
                        } else {
                            app.pending_upids.push(upid.clone());
                            if app.pending_upids.len() <= 5
                                && let Some(client) = &app.client
                            {
                                let client = Arc::clone(client);
                                let tx = tx.clone();
                                let node = app
                                    .current_resource()
                                    .and_then(|r| r.node.clone())
                                    .unwrap_or_default();
                                tokio::spawn(async move {
                                    let mut interval =
                                        tokio::time::interval(Duration::from_secs(2));
                                    loop {
                                        interval.tick().await;
                                        match client.check_task_status(&node, &upid).await {
                                            Ok(TaskStatus::Completed) => {
                                                let _ = tx.send(AppEvent::LifecycleComplete(
                                                    format!("DONE:{}", upid),
                                                ));
                                                break;
                                            }
                                            Ok(TaskStatus::Error) => {
                                                let _ = tx.send(AppEvent::ApiError(format!(
                                                    "Task {} failed",
                                                    upid
                                                )));
                                                break;
                                            }
                                            Ok(TaskStatus::Running) => {
                                                // Continue polling
                                            }
                                            Ok(TaskStatus::Unknown(s)) => {
                                                let _ = tx.send(AppEvent::ApiError(
                                                    format!("Unknown task status: {}", s),
                                                ));
                                                break;
                                            }
                                            Err(e) => {
                                                let _ = tx.send(AppEvent::ApiError(format!(
                                                    "Task poll error: {}",
                                                    e
                                                )));
                                                break;
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                    AppEvent::LifecycleAction(action) => {
                        if let Some(ref client) = app.client {
                            let client = Arc::clone(client);
                            let tx = tx.clone();
                            tokio::spawn(async move {
                                let result = match action {
                                    LifecycleAction::Start { node, vmid, kind } => {
                                        if kind == "qemu" {
                                            client.vm_start(&node, vmid).await
                                        } else if kind == "lxc" {
                                            client.lxc_start(&node, vmid).await
                                        } else {
                                            Err(crate::client::ProxmoxError::Api(format!(
                                                "Unsupported resource type for start: {}",
                                                kind
                                            )))
                                        }
                                    }
                                    LifecycleAction::Stop { node, vmid, kind } => {
                                        if kind == "qemu" {
                                            client.vm_stop(&node, vmid).await
                                        } else if kind == "lxc" {
                                            client.lxc_stop(&node, vmid).await
                                        } else {
                                            Err(crate::client::ProxmoxError::Api(format!(
                                                "Unsupported resource type for stop: {}",
                                                kind
                                            )))
                                        }
                                    }
                                    LifecycleAction::Reboot { node, vmid, kind } => {
                                        if kind == "qemu" {
                                            client.vm_reboot(&node, vmid).await
                                        } else if kind == "lxc" {
                                            client.lxc_reboot(&node, vmid).await
                                        } else {
                                            Err(crate::client::ProxmoxError::Api(format!(
                                                "Unsupported resource type for reboot: {}",
                                                kind
                                            )))
                                        }
                                    }
                                };
                                match result {
                                    Ok(upid) => {
                                        let _ = tx.send(AppEvent::LifecycleComplete(upid));
                                    }
                                    Err(e) => {
                                        let _ = tx.send(AppEvent::ApiError(e.to_string()));
                                    }
                                }
                            });
                        }
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                break;
            }
        }
    }

    tui.leave()?;
    Ok(())
}

fn spawn_polling_task(
    tx: UnboundedSender<AppEvent>,
    client: Arc<ProxmoxClient>,
    interval_secs: u64,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            match client.fetch_resources().await {
                Ok(resources) => {
                    let _ = tx.send(AppEvent::ClusterSnapshot(resources));
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::ApiError(e.to_string()));
                }
            }
        }
    });
}

fn spawn_event_task(tx: UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let mut reader = crossterm::event::EventStream::new();
        loop {
            match reader.next().await {
                Some(Ok(Event::Key(key))) => {
                    let _ = tx.send(AppEvent::Key(key));
                }
                Some(Ok(Event::Resize(w, h))) => {
                    let _ = tx.send(AppEvent::Resize(w, h));
                }
                Some(Ok(_)) => {}
                Some(Err(_)) => break,
                None => break,
            }
        }
    });
}

fn spawn_tick_task(tx: UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(33));
        loop {
            interval.tick().await;
            if tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });
}

fn spawn_version_task(tx: UnboundedSender<AppEvent>, client: Arc<ProxmoxClient>) {
    tokio::spawn(async move {
        match client.fetch_version().await {
            Ok(version) => {
                let _ = tx.send(AppEvent::VersionSnapshot(version.version));
            }
            Err(_) => {
                // Version fetch failed — field stays empty, header shows nothing
            }
        }
    });
}

fn spawn_whoami_task(tx: UnboundedSender<AppEvent>, client: Arc<ProxmoxClient>) {
    tokio::spawn(async move {
        if let Ok(whoami) = client.fetch_whoami().await {
            let _ = tx.send(AppEvent::WhoAmiSnapshot(whoami.username));
        }
    });
}
