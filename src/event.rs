use crossterm::event::KeyEvent;

use crate::client::ClusterResource;

#[derive(Clone, Debug)]
pub enum ConfirmAction {
    Stop {
        node: String,
        vmid: u32,
        kind: String,
    },
    Reboot {
        node: String,
        vmid: u32,
        kind: String,
    },
}

#[derive(Clone, Debug)]
pub enum LifecycleAction {
    Start {
        node: String,
        vmid: u32,
        kind: String,
    },
    Stop {
        node: String,
        vmid: u32,
        kind: String,
    },
    Reboot {
        node: String,
        vmid: u32,
        kind: String,
    },
}

#[derive(Clone, Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
    ClusterSnapshot(Vec<ClusterResource>),
    VersionSnapshot(String),
    WhoAmiSnapshot(String),
    ApiError(String),
    LifecycleComplete(String),
    LifecycleAction(LifecycleAction),
}
