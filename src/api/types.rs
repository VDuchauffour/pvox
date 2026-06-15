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

    /// Format resource details for the details modal overlay.
    pub fn format_details(&self) -> String {
        match self.r#type.as_str() {
            "qemu" | "lxc" => self.format_vm_details(),
            "node" => self.format_node_details(),
            "storage" => self.format_storage_details(),
            _ => self.format_generic_details(),
        }
    }

    fn format_vm_details(&self) -> String {
        let mut s = format!(
            "Name: {}\nType: {}\nNode: {}\nStatus: {}\n\n",
            self.name,
            self.r#type,
            self.node.as_ref().unwrap_or(&"N/A".to_string()),
            self.status
        );
        if let Some(cpu) = self.cpu {
            s.push_str(&format!("CPU: {:.1}%\n", cpu * 100.0));
        }
        if let (Some(mem), Some(maxmem)) = (self.mem, self.maxmem) {
            s.push_str(&format!(
                "Memory: {:.1} / {:.1} GB\n",
                mem as f64 / 1e9,
                maxmem as f64 / 1e9
            ));
        }
        s
    }

    fn format_node_details(&self) -> String {
        format!(
            "Node: {}\nStatus: {}\nCPU: {:.1}%\nMemory: {:.1} / {:.1} GB\nUptime: {}s",
            self.name,
            self.status,
            self.cpu.unwrap_or(0.0) * 100.0,
            self.mem.unwrap_or(0) as f64 / 1e9,
            self.maxmem.unwrap_or(0) as f64 / 1e9,
            self.uptime.unwrap_or(0)
        )
    }

    fn format_storage_details(&self) -> String {
        format!(
            "Storage: {}\nType: {}\nStatus: {}\nDisk: {} / {} GB",
            self.name,
            self.r#type,
            self.status,
            self.disk.unwrap_or(0) / (1024 * 1024 * 1024),
            self.maxdisk.unwrap_or(0) / (1024 * 1024 * 1024)
        )
    }

    fn format_generic_details(&self) -> String {
        format!(
            "Name: {}\nType: {}\nStatus: {}",
            self.name, self.r#type, self.status
        )
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
