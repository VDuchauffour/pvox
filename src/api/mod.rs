mod client;
mod error;
mod types;

pub use client::ProxmoxClient;
pub use error::ProxmoxError;
pub use types::{ClusterResource, PveVersion, RrdDataPoint, TaskStatus, WhoAmI};
