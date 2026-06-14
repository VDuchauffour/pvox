use crate::client::ClusterResource;
use crossterm::event::KeyEvent;

#[derive(Clone, Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
    ClusterSnapshot(Vec<ClusterResource>),
    ApiError(String),
    LifecycleComplete(String),
}
