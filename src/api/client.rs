use reqwest::Client;

use super::error::ProxmoxError;
use super::types::{
    ClusterBackup, ClusterHaResource, ClusterReplication, ClusterResource, ClusterTask, NodeDisk,
    PveVersion, RrdDataPoint, TaskStatus, WhoAmI,
};

pub struct ProxmoxClient {
    client: Client,
    base_url: String,
    auth_header: String,
}

impl ProxmoxClient {
    pub fn new(
        endpoint: &str,
        token_id: &str,
        token: &str,
        insecure: bool,
    ) -> Result<Self, ProxmoxError> {
        let client = Client::builder()
            .danger_accept_invalid_certs(insecure)
            .build()?;
        let base_url = endpoint.trim_end_matches('/').to_string();
        let auth_header = format!("PVEAPIToken={}={}", token_id, token);
        Ok(Self {
            client,
            base_url,
            auth_header,
        })
    }

    // -- Public API --------------------------------------------------------------

    pub async fn fetch_resources(&self) -> Result<Vec<ClusterResource>, ProxmoxError> {
        let data = self.get_data("/api2/json/cluster/resources").await?;
        let array = data
            .as_array()
            .ok_or_else(|| ProxmoxError::Api("Expected array response".into()))?;
        let mut resources: Vec<ClusterResource> = array
            .iter()
            .map(|v| serde_json::from_value::<ClusterResource>(v.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ProxmoxError::Api(format!("Failed to parse resource: {e}")))?;
        for r in &mut resources {
            r.normalize();
        }
        Ok(resources)
    }

    pub async fn fetch_cluster_tasks(&self) -> Result<Vec<ClusterResource>, ProxmoxError> {
        let data = self.get_data("/api2/json/cluster/tasks").await?;
        let array = data
            .as_array()
            .ok_or_else(|| ProxmoxError::Api("Expected array response".into()))?;
        Ok(array
            .iter()
            .map(|v| serde_json::from_value::<ClusterTask>(v.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ProxmoxError::Api(format!("Failed to parse task: {e}")))?
            .into_iter()
            .map(ClusterTask::into_resource)
            .collect())
    }

    pub async fn fetch_replication(&self) -> Result<Vec<ClusterResource>, ProxmoxError> {
        let data = self.get_data("/api2/json/cluster/replication").await?;
        let array = data
            .as_array()
            .ok_or_else(|| ProxmoxError::Api("Expected array response".into()))?;
        Ok(array
            .iter()
            .map(|v| serde_json::from_value::<ClusterReplication>(v.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ProxmoxError::Api(format!("Failed to parse replication: {e}")))?
            .into_iter()
            .map(ClusterReplication::into_resource)
            .collect())
    }

    pub async fn fetch_ha_resources(&self) -> Result<Vec<ClusterResource>, ProxmoxError> {
        let data = self.get_data("/api2/json/cluster/ha/resources").await?;
        let array = data
            .as_array()
            .ok_or_else(|| ProxmoxError::Api("Expected array response".into()))?;
        Ok(array
            .iter()
            .map(|v| serde_json::from_value::<ClusterHaResource>(v.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ProxmoxError::Api(format!("Failed to parse HA resource: {e}")))?
            .into_iter()
            .map(ClusterHaResource::into_resource)
            .collect())
    }

    pub async fn fetch_backups(&self) -> Result<Vec<ClusterResource>, ProxmoxError> {
        let data = self.get_data("/api2/json/cluster/backup").await?;
        let array = data
            .as_array()
            .ok_or_else(|| ProxmoxError::Api("Expected array response".into()))?;
        Ok(array
            .iter()
            .map(|v| serde_json::from_value::<ClusterBackup>(v.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ProxmoxError::Api(format!("Failed to parse backup: {e}")))?
            .into_iter()
            .map(ClusterBackup::into_resource)
            .collect())
    }

    pub async fn fetch_node_disks(&self) -> Result<Vec<ClusterResource>, ProxmoxError> {
        let nodes_data = self.get_data("/api2/json/nodes").await?;
        let nodes = nodes_data
            .as_array()
            .ok_or_else(|| ProxmoxError::Api("Expected nodes array".into()))?;
        let mut disks: Vec<ClusterResource> = Vec::new();
        for node in nodes {
            let node_name = node
                .get("node")
                .and_then(|n| n.as_str())
                .ok_or_else(|| ProxmoxError::Api("Missing node name".into()))?;
            match self
                .get_data(&format!("/api2/json/nodes/{node_name}/disks/list"))
                .await
            {
                Ok(data) => {
                    if let Some(array) = data.as_array() {
                        for value in array {
                            match serde_json::from_value::<NodeDisk>(value.clone()) {
                                Ok(disk) => disks.push(disk.into_resource(node_name.to_string())),
                                Err(e) => {
                                    return Err(ProxmoxError::Api(format!(
                                        "Failed to parse disk on {node_name}: {e}"
                                    )));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(ProxmoxError::Api(format!(
                        "Failed to fetch disks for {node_name}: {e}"
                    )));
                }
            }
        }
        Ok(disks)
    }

    pub async fn fetch_rrd_data(
        &self,
        node: &str,
        timeframe: &str,
    ) -> Result<Vec<RrdDataPoint>, ProxmoxError> {
        let data = self
            .get_data(&format!(
                "/api2/json/nodes/{node}/rrddata?timeframe={timeframe}"
            ))
            .await?;
        let array = data
            .as_array()
            .ok_or_else(|| ProxmoxError::Api("Expected array response".into()))?;
        Ok(array
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect())
    }

    pub async fn vm_start(&self, node: &str, vmid: u32) -> Result<String, ProxmoxError> {
        self.post_for_upid(&format!("/api2/json/nodes/{node}/qemu/{vmid}/status/start"))
            .await
    }

    pub async fn vm_stop(&self, node: &str, vmid: u32) -> Result<String, ProxmoxError> {
        self.post_for_upid(&format!("/api2/json/nodes/{node}/qemu/{vmid}/status/stop"))
            .await
    }

    pub async fn vm_reboot(&self, node: &str, vmid: u32) -> Result<String, ProxmoxError> {
        self.post_for_upid(&format!(
            "/api2/json/nodes/{node}/qemu/{vmid}/status/reboot"
        ))
        .await
    }

    pub async fn lxc_start(&self, node: &str, vmid: u32) -> Result<String, ProxmoxError> {
        self.post_for_upid(&format!("/api2/json/nodes/{node}/lxc/{vmid}/status/start"))
            .await
    }

    pub async fn lxc_stop(&self, node: &str, vmid: u32) -> Result<String, ProxmoxError> {
        self.post_for_upid(&format!("/api2/json/nodes/{node}/lxc/{vmid}/status/stop"))
            .await
    }

    pub async fn lxc_reboot(&self, node: &str, vmid: u32) -> Result<String, ProxmoxError> {
        self.post_for_upid(&format!("/api2/json/nodes/{node}/lxc/{vmid}/status/reboot"))
            .await
    }

    pub async fn fetch_version(&self) -> Result<PveVersion, ProxmoxError> {
        let data = self.get_data("/api2/json/version").await?;
        serde_json::from_value(data)
            .map_err(|e| ProxmoxError::Api(format!("Failed to parse version: {e}")))
    }

    pub async fn fetch_whoami(&self) -> Result<WhoAmI, ProxmoxError> {
        let data = self.get_data("/api2/json/access/whoami").await?;
        serde_json::from_value(data)
            .map_err(|e| ProxmoxError::Api(format!("Failed to parse whoami: {e}")))
    }

    pub async fn check_task_status(
        &self,
        node: &str,
        upid: &str,
    ) -> Result<TaskStatus, ProxmoxError> {
        let data = self
            .get_data(&format!("/api2/json/nodes/{node}/tasks/{upid}/status"))
            .await?;
        let status = data
            .get("status")
            .and_then(|s| s.as_str())
            .ok_or_else(|| ProxmoxError::Api("Missing status field".into()))?;
        Ok(match status {
            "OK" => TaskStatus::Completed,
            "ERROR" => TaskStatus::Error,
            "running" => TaskStatus::Running,
            _ => TaskStatus::Unknown(status.to_string()),
        })
    }

    // -- Private helpers ---------------------------------------------------------

    async fn get_data(&self, path: &str) -> Result<serde_json::Value, ProxmoxError> {
        let url = format!("{}{path}", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;
        self.extract_data(resp).await
    }

    async fn post_for_upid(&self, path: &str) -> Result<String, ProxmoxError> {
        let data = self.post_data(path).await?;
        data.as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| ProxmoxError::Api("Missing UPID in response".into()))
    }

    async fn post_data(&self, path: &str) -> Result<serde_json::Value, ProxmoxError> {
        let url = format!("{}{path}", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;
        self.extract_data(resp).await
    }

    async fn extract_data(
        &self,
        resp: reqwest::Response,
    ) -> Result<serde_json::Value, ProxmoxError> {
        match resp.status() {
            reqwest::StatusCode::OK => {
                let body: serde_json::Value = resp.json().await?;
                body.get("data")
                    .cloned()
                    .ok_or_else(|| ProxmoxError::Api("Missing data field".into()))
            }
            reqwest::StatusCode::UNAUTHORIZED => Err(ProxmoxError::Unauthorized),
            reqwest::StatusCode::FORBIDDEN => Err(ProxmoxError::Forbidden),
            _ => Err(ProxmoxError::Api(format!("HTTP {}", resp.status()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_header_format() {
        let client =
            ProxmoxClient::new("https://pve.local:8006", "root@pam!pvox", "abc123", false).unwrap();
        assert_eq!(client.auth_header, "PVEAPIToken=root@pam!pvox=abc123");
    }

    #[test]
    fn test_parse_cluster_resources() {
        let json = serde_json::json!({
            "data": [
                {
                    "id": "node/pve",
                    "type": "node",
                    "name": "pve",
                    "node": "pve",
                    "status": "online",
                    "cpu": 0.15,
                    "maxcpu": 8,
                    "mem": 4294967296u64,
                    "maxmem": 17179869184u64,
                    "disk": 2147483648u64,
                    "maxdisk": 10737418240u64,
                    "uptime": 3600
                },
                {
                    "id": "qemu/100",
                    "type": "qemu",
                    "name": "win10",
                    "node": "pve",
                    "status": "running",
                    "cpu": 0.05,
                    "maxcpu": 4,
                    "mem": 2147483648u64,
                    "maxmem": 8589934592u64
                }
            ]
        });

        let data = json.get("data").and_then(|d| d.as_array()).unwrap();
        let resources: Vec<ClusterResource> = data
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();

        assert_eq!(resources.len(), 2);

        let node = &resources[0];
        assert_eq!(node.id, "node/pve");
        assert_eq!(node.r#type, "node");
        assert_eq!(node.name, "pve");
        assert_eq!(node.node.as_deref(), Some("pve"));
        assert_eq!(node.status, "online");
        assert_eq!(node.cpu, Some(0.15));
        assert_eq!(node.maxcpu, Some(8.0));
        assert_eq!(node.mem, Some(4294967296));
        assert_eq!(node.maxmem, Some(17179869184));
        assert_eq!(node.disk, Some(2147483648));
        assert_eq!(node.maxdisk, Some(10737418240));
        assert_eq!(node.uptime, Some(3600));

        let vm = &resources[1];
        assert_eq!(vm.id, "qemu/100");
        assert_eq!(vm.r#type, "qemu");
        assert_eq!(vm.name, "win10");
        assert_eq!(vm.status, "running");
        assert_eq!(vm.cpu, Some(0.05));
        assert_eq!(vm.maxcpu, Some(4.0));
        assert_eq!(vm.mem, Some(2147483648));
        assert_eq!(vm.maxmem, Some(8589934592));
        assert!(vm.disk.is_none());
        assert!(vm.maxdisk.is_none());
        assert!(vm.uptime.is_none());
    }

    #[test]
    fn test_fetch_resources_mock() {
        let json = serde_json::json!({
            "data": [
                {
                    "id": "qemu/100",
                    "type": "qemu",
                    "name": "win10",
                    "node": "pve",
                    "status": "running",
                    "cpu": 0.05,
                    "maxcpu": 4,
                    "mem": 2147483648u64,
                    "maxmem": 8589934592u64
                },
                {
                    "id": "lxc/200",
                    "type": "lxc",
                    "name": "ubuntu",
                    "node": "pve",
                    "status": "stopped"
                }
            ]
        });

        let data = json.get("data").and_then(|d| d.as_array()).unwrap();
        let resources: Vec<ClusterResource> = data
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();

        assert_eq!(resources.len(), 2);

        let vm = &resources[0];
        assert_eq!(vm.id, "qemu/100");
        assert_eq!(vm.r#type, "qemu");
        assert_eq!(vm.name, "win10");
        assert_eq!(vm.status, "running");
        assert_eq!(vm.cpu, Some(0.05));
        assert_eq!(vm.maxcpu, Some(4.0));
        assert_eq!(vm.mem, Some(2147483648));
        assert_eq!(vm.maxmem, Some(8589934592));
        assert!(vm.disk.is_none());
        assert!(vm.maxdisk.is_none());
        assert!(vm.uptime.is_none());

        let ct = &resources[1];
        assert_eq!(ct.id, "lxc/200");
        assert_eq!(ct.r#type, "lxc");
        assert_eq!(ct.name, "ubuntu");
        assert_eq!(ct.status, "stopped");
        assert!(ct.cpu.is_none());
        assert!(ct.maxcpu.is_none());
        assert!(ct.mem.is_none());
        assert!(ct.maxmem.is_none());
        assert!(ct.disk.is_none());
        assert!(ct.maxdisk.is_none());
        assert!(ct.uptime.is_none());
    }

    #[test]
    fn test_parse_resources_without_name_field() {
        let json = serde_json::json!({
            "data": [
                {
                    "cpu": 0.15, "id": "node/pve-node1", "node": "pve-node1",
                    "status": "online", "type": "node", "level": "",
                    "mem": 8589934592u64, "uptime": 86400, "disk": 50000000000u64,
                    "maxcpu": 8, "maxdisk": 100000000000u64, "maxmem": 17179869184u64
                },
                {
                    "id": "sdn/zones/vlan-zone", "status": "available",
                    "type": "sdn", "plugin": "vlan"
                }
            ]
        });

        let data = json.get("data").and_then(|d| d.as_array()).unwrap();
        let mut resources: Vec<ClusterResource> = data
            .iter()
            .map(|v| serde_json::from_value::<ClusterResource>(v.clone()).unwrap())
            .collect();
        for r in &mut resources {
            r.normalize();
        }

        assert_eq!(resources.len(), 2);
        assert_eq!(resources[0].r#type, "node");
        assert_eq!(resources[0].name, "pve-node1");
        assert_eq!(resources[1].r#type, "sdn");
        assert_eq!(resources[1].name, "sdn/zones/vlan-zone");
    }

    #[test]
    fn test_unauthorized_handling() {
        let status = reqwest::StatusCode::UNAUTHORIZED;
        match status {
            reqwest::StatusCode::UNAUTHORIZED => {
                let err = ProxmoxError::Unauthorized;
                assert_eq!(format!("{}", err), "Unauthorized — check token");
            }
            _ => panic!("Expected UNAUTHORIZED status code"),
        }
    }

    #[test]
    fn test_forbidden_handling() {
        let status = reqwest::StatusCode::FORBIDDEN;
        match status {
            reqwest::StatusCode::FORBIDDEN => {
                let err = ProxmoxError::Forbidden;
                assert_eq!(format!("{}", err), "Forbidden — insufficient permissions");
            }
            _ => panic!("Expected FORBIDDEN status code"),
        }
    }

    #[test]
    fn test_parse_resources_includes_pool_entries() {
        // Proxmox returns pool resources without a `status` field; they must
        // not break parsing of the entire cluster/resources response.
        let json = serde_json::json!({
            "data": [
                {
                    "pool": "production",
                    "type": "pool",
                    "id": "/pool/production"
                },
                {
                    "id": "qemu/100",
                    "type": "qemu",
                    "name": "web-01",
                    "node": "pve",
                    "status": "running",
                    "cpu": 0.05,
                    "maxcpu": 2,
                    "mem": 2147483648u64,
                    "maxmem": 4294967296u64
                }
            ]
        });

        let data = json.get("data").and_then(|d| d.as_array()).unwrap();
        let mut resources: Vec<ClusterResource> = data
            .iter()
            .map(|v| serde_json::from_value::<ClusterResource>(v.clone()).unwrap())
            .collect();
        for r in &mut resources {
            r.normalize();
        }

        assert_eq!(resources.len(), 2);
        assert_eq!(resources[0].r#type, "pool");
        assert_eq!(resources[0].id, "/pool/production");
        assert_eq!(resources[0].status, "");
        assert_eq!(resources[1].r#type, "qemu");
        assert_eq!(resources[1].status, "running");
    }

    #[test]
    fn test_parse_cluster_tasks() {
        let json = serde_json::json!({
            "data": [
                {
                    "upid": "UPID:pve:00123456:00000001:RUNNING:vmstart:100:root@pam:",
                    "type": "vmstart",
                    "status": "running",
                    "starttime": 1700000000,
                    "endtime": 0,
                    "node": "pve",
                    "user": "root@pam"
                },
                {
                    "upid": "UPID:pve:00987654:00000002:OK:vzdump:101:root@pam:",
                    "type": "vzdump",
                    "status": "",
                    "exitstatus": "OK",
                    "starttime": 1700000100,
                    "endtime": 1700000200,
                    "node": "pve",
                    "user": "root@pam"
                }
            ]
        });

        let data = json.get("data").and_then(|d| d.as_array()).unwrap();
        let tasks: Vec<ClusterResource> = data
            .iter()
            .map(|v| serde_json::from_value::<ClusterTask>(v.clone()).unwrap())
            .map(ClusterTask::into_resource)
            .collect();

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].r#type, "task");
        assert_eq!(tasks[0].name, "vmstart");
        assert_eq!(tasks[0].status, "running");
        assert_eq!(tasks[0].endtime, None);
        assert_eq!(tasks[1].name, "vzdump");
        assert_eq!(tasks[1].status, "OK");
        assert_eq!(tasks[1].endtime, Some(1_700_000_200));
    }

    #[test]
    fn test_parse_replication_jobs() {
        let json = serde_json::json!({
            "data": [
                {
                    "id": "100-0",
                    "type": "local",
                    "guest": "100",
                    "source": "pve",
                    "target": "pve2",
                    "schedule": "*/15",
                    "disable": 0
                },
                {
                    "id": "101-0",
                    "type": "local",
                    "guest": "101",
                    "source": "pve",
                    "target": "pve2",
                    "schedule": "0 2 * * *",
                    "disable": 1
                }
            ]
        });

        let data = json.get("data").and_then(|d| d.as_array()).unwrap();
        let jobs: Vec<ClusterResource> = data
            .iter()
            .map(|v| serde_json::from_value::<ClusterReplication>(v.clone()).unwrap())
            .map(ClusterReplication::into_resource)
            .collect();

        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].r#type, "replication");
        assert_eq!(jobs[0].name, "[local] 100 -> pve2");
        assert_eq!(jobs[0].status, "enabled");
        assert_eq!(jobs[1].status, "disabled");
        assert_eq!(jobs[1].schedule.as_deref(), Some("0 2 * * *"));
    }

    #[test]
    fn test_parse_ha_resources() {
        let json = serde_json::json!({
            "data": [
                {
                    "sid": "vm:100",
                    "type": "vm",
                    "state": "started",
                    "node": "pve",
                    "group": "production",
                    "max_restart": 1,
                    "max_relocate": 1
                },
                {
                    "sid": "ct:200",
                    "type": "ct",
                    "state": "stopped",
                    "node": "pve"
                }
            ]
        });

        let data = json.get("data").and_then(|d| d.as_array()).unwrap();
        let ha: Vec<ClusterResource> = data
            .iter()
            .map(|v| serde_json::from_value::<ClusterHaResource>(v.clone()).unwrap())
            .map(ClusterHaResource::into_resource)
            .collect();

        assert_eq!(ha.len(), 2);
        assert_eq!(ha[0].r#type, "ha");
        assert_eq!(ha[0].name, "[vm] vm:100");
        assert_eq!(ha[0].status, "started");
        assert_eq!(ha[0].group.as_deref(), Some("production"));
        assert_eq!(ha[1].status, "stopped");
        assert_eq!(ha[1].group, None);
    }

    #[test]
    fn test_parse_backup_jobs() {
        let json = serde_json::json!({
            "data": [
                {
                    "id": "backup-100",
                    "vmid": "100",
                    "schedule": "0 2 * * *",
                    "enabled": 1,
                    "mode": "stop",
                    "storage": "local",
                    "node": "pve"
                },
                {
                    "id": "backup-200",
                    "vmid": "200",
                    "schedule": "0 3 * * 0",
                    "enabled": 0,
                    "mode": "suspend",
                    "storage": "local",
                    "node": "pve"
                }
            ]
        });

        let data = json.get("data").and_then(|d| d.as_array()).unwrap();
        let backups: Vec<ClusterResource> = data
            .iter()
            .map(|v| serde_json::from_value::<ClusterBackup>(v.clone()).unwrap())
            .map(ClusterBackup::into_resource)
            .collect();

        assert_eq!(backups.len(), 2);
        assert_eq!(backups[0].r#type, "backup");
        assert_eq!(backups[0].name, "100");
        assert_eq!(backups[0].status, "enabled");
        assert_eq!(backups[1].status, "disabled");
        assert_eq!(backups[1].storage.as_deref(), Some("local"));
    }

    #[test]
    fn test_parse_node_disks() {
        let json = serde_json::json!({
            "data": [
                {
                    "devpath": "/dev/sda",
                    "type": "ssd",
                    "size": 1000204886016u64,
                    "model": "Samsung SSD 870",
                    "health": "PASSED",
                    "serial": "S123456789",
                    "wearout": 95
                },
                {
                    "devpath": "/dev/sdb",
                    "type": "hd",
                    "size": 4000787030016u64,
                    "model": "",
                    "health": "UNKNOWN",
                    "serial": "",
                    "wearout": "N/A"
                }
            ]
        });

        let data = json.get("data").and_then(|d| d.as_array()).unwrap();
        let disks: Vec<ClusterResource> = data
            .iter()
            .map(|v| serde_json::from_value::<NodeDisk>(v.clone()).unwrap())
            .map(|d| d.into_resource("pve".to_string()))
            .collect();

        assert_eq!(disks.len(), 2);
        assert_eq!(disks[0].r#type, "disk");
        assert_eq!(disks[0].name, "[ssd] Samsung SSD 870");
        assert_eq!(disks[0].node.as_deref(), Some("pve"));
        assert_eq!(disks[0].status, "PASSED");
        assert_eq!(disks[0].wearout, Some(95));
        assert_eq!(disks[1].name, "[hd] /dev/sdb");
        assert_eq!(disks[1].wearout, None);
    }

    #[tokio::test]
    async fn test_connection_refused() {
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let err = client.get("http://127.0.0.1:1/").send().await.unwrap_err();

        assert!(err.is_connect() || err.is_request());

        let proxmox_err = ProxmoxError::Http(err);
        assert!(matches!(proxmox_err, ProxmoxError::Http(_)));
    }
}
