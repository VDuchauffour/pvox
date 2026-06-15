use serde::Deserialize;

/// Proxmox returns booleans as integers (`0`/`1`) on most endpoints; accept
/// bool, integer, or string forms.
fn de_flexible_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::Bool(b) => b,
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
        serde_json::Value::String(s) => s == "1" || s.eq_ignore_ascii_case("true"),
        _ => false,
    })
}

/// Proxmox disk `wearout` is an integer for SSDs but the string `"N/A"` for
/// spinning disks; coerce anything non-numeric to `None`.
fn de_flexible_wearout<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::Number(n) => n.as_u64().map(|x| x as u32),
        serde_json::Value::String(s) => s.parse::<u32>().ok(),
        _ => None,
    })
}

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
    #[serde(default)]
    pub status: String,
    pub cpu: Option<f64>,
    pub maxcpu: Option<f64>,
    pub mem: Option<u64>,
    pub maxmem: Option<u64>,
    pub disk: Option<u64>,
    pub maxdisk: Option<u64>,
    pub uptime: Option<u64>,
    #[serde(default)]
    pub starttime: Option<u64>,
    #[serde(default)]
    pub endtime: Option<u64>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub schedule: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub disable: Option<bool>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub max_restart: Option<u32>,
    #[serde(default)]
    pub max_relocate: Option<u32>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub storage: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub serial: Option<String>,
    #[serde(default)]
    pub wearout: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClusterTask {
    pub upid: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub exitstatus: String,
    pub starttime: u64,
    #[serde(default)]
    pub endtime: u64,
    pub node: String,
    #[serde(default)]
    pub user: String,
}

impl ClusterTask {
    pub fn into_resource(self) -> ClusterResource {
        ClusterResource {
            id: self.upid,
            r#type: "task".to_string(),
            name: self.r#type.clone(),
            node: Some(self.node),
            status: if self.status.is_empty() {
                self.exitstatus.clone()
            } else {
                self.status.clone()
            },
            cpu: None,
            maxcpu: None,
            mem: None,
            maxmem: None,
            disk: None,
            maxdisk: None,
            uptime: None,
            starttime: Some(self.starttime),
            endtime: if self.endtime == 0 {
                None
            } else {
                Some(self.endtime)
            },
            user: Some(self.user),
            schedule: None,
            target: None,
            disable: None,
            group: None,
            max_restart: None,
            max_relocate: None,
            enabled: None,
            storage: None,
            mode: None,
            model: None,
            serial: None,
            wearout: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClusterReplication {
    pub id: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub guest: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub schedule: String,
    #[serde(default, deserialize_with = "de_flexible_bool")]
    pub disable: bool,
}

impl ClusterReplication {
    pub fn into_resource(self) -> ClusterResource {
        ClusterResource {
            id: self.id.clone(),
            r#type: "replication".to_string(),
            name: format!("[{}] {} -> {}", self.r#type, self.guest, self.target),
            node: Some(self.source.clone()),
            status: if self.disable {
                "disabled".to_string()
            } else {
                "enabled".to_string()
            },
            cpu: None,
            maxcpu: None,
            mem: None,
            maxmem: None,
            disk: None,
            maxdisk: None,
            uptime: None,
            starttime: None,
            endtime: None,
            user: None,
            schedule: Some(self.schedule),
            target: Some(self.target),
            disable: Some(self.disable),
            group: None,
            max_restart: None,
            max_relocate: None,
            enabled: None,
            storage: None,
            mode: None,
            model: None,
            serial: None,
            wearout: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClusterHaResource {
    pub sid: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub node: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub max_restart: u32,
    #[serde(default)]
    pub max_relocate: u32,
}

impl ClusterHaResource {
    pub fn into_resource(self) -> ClusterResource {
        ClusterResource {
            id: self.sid.clone(),
            r#type: "ha".to_string(),
            name: format!("[{}] {}", self.r#type, self.sid),
            node: Some(self.node.clone()),
            status: self.state.clone(),
            cpu: None,
            maxcpu: None,
            mem: None,
            maxmem: None,
            disk: None,
            maxdisk: None,
            uptime: None,
            starttime: None,
            endtime: None,
            user: None,
            schedule: None,
            target: None,
            disable: None,
            group: if self.group.is_empty() {
                None
            } else {
                Some(self.group)
            },
            max_restart: if self.max_restart == 0 {
                None
            } else {
                Some(self.max_restart)
            },
            max_relocate: if self.max_relocate == 0 {
                None
            } else {
                Some(self.max_relocate)
            },
            enabled: None,
            storage: None,
            mode: None,
            model: None,
            serial: None,
            wearout: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClusterBackup {
    pub id: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub schedule: String,
    #[serde(default, deserialize_with = "de_flexible_bool")]
    pub enabled: bool,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub storage: String,
    #[serde(default)]
    pub node: String,
    #[serde(default)]
    pub vmid: String,
}

impl ClusterBackup {
    pub fn into_resource(self) -> ClusterResource {
        ClusterResource {
            id: self.id.clone(),
            r#type: "backup".to_string(),
            name: match (self.r#type.is_empty(), self.vmid.is_empty()) {
                (true, true) => "all".to_string(),
                (true, false) => self.vmid.clone(),
                (false, _) => format!("[{}] {}", self.r#type, self.vmid),
            },
            node: if self.node.is_empty() {
                None
            } else {
                Some(self.node.clone())
            },
            status: if self.enabled {
                "enabled".to_string()
            } else {
                "disabled".to_string()
            },
            cpu: None,
            maxcpu: None,
            mem: None,
            maxmem: None,
            disk: None,
            maxdisk: None,
            uptime: None,
            starttime: None,
            endtime: None,
            user: None,
            schedule: if self.schedule.is_empty() {
                None
            } else {
                Some(self.schedule)
            },
            target: None,
            disable: Some(!self.enabled),
            group: None,
            max_restart: None,
            max_relocate: None,
            enabled: Some(self.enabled),
            storage: if self.storage.is_empty() {
                None
            } else {
                Some(self.storage)
            },
            mode: if self.mode.is_empty() {
                None
            } else {
                Some(self.mode)
            },
            model: None,
            serial: None,
            wearout: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct NodeDisk {
    pub devpath: String,
    #[serde(default)]
    pub r#type: String,
    pub size: u64,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub health: String,
    #[serde(default)]
    pub serial: String,
    #[serde(default, deserialize_with = "de_flexible_wearout")]
    pub wearout: Option<u32>,
}

impl NodeDisk {
    pub fn into_resource(self, node: String) -> ClusterResource {
        ClusterResource {
            id: self.devpath.clone(),
            r#type: "disk".to_string(),
            name: if self.model.is_empty() {
                format!("[{}] {}", self.r#type, self.devpath)
            } else {
                format!("[{}] {}", self.r#type, self.model)
            },
            node: Some(node),
            status: if self.health.is_empty() {
                "unknown".to_string()
            } else {
                self.health.clone()
            },
            cpu: None,
            maxcpu: None,
            mem: None,
            maxmem: None,
            disk: Some(self.size),
            maxdisk: None,
            uptime: None,
            starttime: None,
            endtime: None,
            user: None,
            schedule: None,
            target: None,
            disable: None,
            group: None,
            max_restart: None,
            max_relocate: None,
            enabled: None,
            storage: None,
            mode: None,
            model: if self.model.is_empty() {
                None
            } else {
                Some(self.model)
            },
            serial: if self.serial.is_empty() {
                None
            } else {
                Some(self.serial)
            },
            wearout: self.wearout,
        }
    }
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
            "task" => self.format_task_details(),
            "replication" => self.format_replication_details(),
            "ha" => self.format_ha_details(),
            "backup" => self.format_backup_details(),
            "disk" => self.format_disk_details(),
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

    fn format_task_details(&self) -> String {
        let mut s = format!(
            "Task: {}\nNode: {}\nStatus: {}",
            self.name,
            self.node.as_ref().unwrap_or(&"N/A".to_string()),
            self.status
        );
        if let Some(user) = &self.user {
            s.push_str(&format!("\nUser: {user}"));
        }
        if let Some(start) = self.starttime {
            s.push_str(&format!("\nStarted: {start}"));
        }
        if let Some(end) = self.endtime {
            s.push_str(&format!("\nEnded: {end}"));
        }
        s
    }

    fn format_replication_details(&self) -> String {
        let mut s = format!(
            "Replication: {}\nNode: {}\nStatus: {}",
            self.name,
            self.node.as_ref().unwrap_or(&"N/A".to_string()),
            self.status
        );
        if let Some(target) = &self.target {
            s.push_str(&format!("\nTarget: {target}"));
        }
        if let Some(schedule) = &self.schedule {
            s.push_str(&format!("\nSchedule: {schedule}"));
        }
        s
    }

    fn format_ha_details(&self) -> String {
        let mut s = format!(
            "HA Resource: {}\nNode: {}\nState: {}",
            self.name,
            self.node.as_ref().unwrap_or(&"N/A".to_string()),
            self.status
        );
        if let Some(group) = &self.group {
            s.push_str(&format!("\nGroup: {group}"));
        }
        if let Some(max_restart) = self.max_restart {
            s.push_str(&format!("\nMax restart: {max_restart}"));
        }
        if let Some(max_relocate) = self.max_relocate {
            s.push_str(&format!("\nMax relocate: {max_relocate}"));
        }
        s
    }

    fn format_backup_details(&self) -> String {
        let mut s = format!(
            "Backup: {}\nNode: {}\nStatus: {}",
            self.name,
            self.node.as_ref().unwrap_or(&"N/A".to_string()),
            self.status
        );
        if let Some(schedule) = &self.schedule {
            s.push_str(&format!("\nSchedule: {schedule}"));
        }
        if let Some(storage) = &self.storage {
            s.push_str(&format!("\nStorage: {storage}"));
        }
        if let Some(mode) = &self.mode {
            s.push_str(&format!("\nMode: {mode}"));
        }
        s
    }

    fn format_disk_details(&self) -> String {
        let mut s = format!(
            "Disk: {}\nNode: {}\nHealth: {}",
            self.name,
            self.node.as_ref().unwrap_or(&"N/A".to_string()),
            self.status
        );
        if let Some(size) = self.disk {
            s.push_str(&format!("\nSize: {} GB", size / (1024 * 1024 * 1024)));
        }
        if let Some(model) = &self.model {
            s.push_str(&format!("\nModel: {model}"));
        }
        if let Some(serial) = &self.serial {
            s.push_str(&format!("\nSerial: {serial}"));
        }
        if let Some(wearout) = self.wearout {
            s.push_str(&format!("\nWearout: {wearout}%"));
        }
        s
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
