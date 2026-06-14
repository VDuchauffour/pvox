pub mod app;
pub mod client;
pub mod config;
pub mod event;
pub mod tui;
pub mod ui;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::Event;
use futures::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::App;
use crate::client::ProxmoxClient;
use crate::config::Config;
use crate::event::AppEvent;
use crate::tui::Tui;

pub async fn run(config: Config) -> Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();

    let mut tui = Tui::new()?;
    let mut app = App::new(config)?;

    let client_arc = app.client.take().map(Arc::new);
    if let Some(ref client) = client_arc {
        spawn_polling_task(tx.clone(), client.clone(), app.config.refresh_interval);
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
                        app.handle_key(key);
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
                    AppEvent::ApiError(err) => {
                        app.connected = false;
                        app.status_message = Some(err);
                    }
                    _ => {}
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

fn spawn_polling_task(tx: UnboundedSender<AppEvent>, client: Arc<ProxmoxClient>, interval_secs: u64) {
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
