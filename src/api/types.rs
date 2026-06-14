use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct PveVersion {
    pub version: String,
    #[serde(default)]
    pub release: String,
    #[serde(default)]
    pub repoid: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WhoAmI {
    pub username: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClusterResource {
    pub id: String,
    pub r#type: String,
    #[serde(default)]
    pub name: String,
    pub node: Option<String>,
    pub status: String,
    pub cpu: Option<f64>,
    pub maxcpu: Option<f64>,
    pub mem: Option<u64>,
    pub maxmem: Option<u64>,
    pub disk: Option<u64>,
    pub maxdisk: Option<u64>,
    pub uptime: Option<u64>,
}

impl ClusterResource {
    /// Proxmox omits `name` for node and SDN resources; fall back to node name or id.
    pub fn normalize(&mut self) {
        if self.name.is_empty() {
            self.name = self.node.clone().unwrap_or_else(|| self.id.clone());
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RrdDataPoint {
    pub time: u64,
    pub cpu: Option<f64>,
    pub mem: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum TaskStatus {
    Running,
    Completed,
    Error,
    Unknown(String),
}
