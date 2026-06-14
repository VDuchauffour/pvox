use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;

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

#[derive(Debug, Deserialize)]
pub struct RrdDataPoint {
    pub time: u64,
    pub cpu: Option<f64>,
    pub mem: Option<f64>,
}

#[derive(Debug, Error)]
pub enum ProxmoxError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Unauthorized — check token")]
    Unauthorized,
    #[error("Forbidden — insufficient permissions")]
    Forbidden,
}

#[derive(Debug, Clone)]
pub enum TaskStatus {
    Running,
    Completed,
    Error,
    Unknown(String),
}

pub struct ProxmoxClient {
    client: Client,
    base_url: String,
    auth_header: String,
}

impl ProxmoxClient {
    pub fn new(
        host: &str,
        token_id: &str,
        token: &str,
        insecure: bool,
    ) -> Result<Self, ProxmoxError> {
        let client = Client::builder()
            .danger_accept_invalid_certs(insecure)
            .build()?;
        let base_url = host.trim_end_matches('/').to_string();
        let auth_header = format!("PVEAPIToken={}={}", token_id, token);
        Ok(Self {
            client,
            base_url,
            auth_header,
        })
    }

    pub async fn fetch_resources(&self) -> Result<Vec<ClusterResource>, ProxmoxError> {
        let url = format!("{}/api2/json/cluster/resources", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        match resp.status() {
            reqwest::StatusCode::OK => {
                let body: serde_json::Value = resp.json().await?;
                let data = body
                    .get("data")
                    .and_then(|d| d.as_array())
                    .ok_or_else(|| ProxmoxError::Api("Missing data field".into()))?;
                let resources: Vec<ClusterResource> = data
                    .iter()
                    .map(|v| serde_json::from_value::<ClusterResource>(v.clone()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| ProxmoxError::Api(format!("Failed to parse resource: {e}")))?
                    .into_iter()
                    .map(|mut r| {
                        if r.name.is_empty() {
                            r.name = r.node.clone().unwrap_or_else(|| r.id.clone());
                        }
                        r
                    })
                    .collect();
                Ok(resources)
            }
            reqwest::StatusCode::UNAUTHORIZED => Err(ProxmoxError::Unauthorized),
            reqwest::StatusCode::FORBIDDEN => Err(ProxmoxError::Forbidden),
            _ => Err(ProxmoxError::Api(format!("HTTP {}", resp.status()))),
        }
    }

    pub async fn fetch_rrd_data(
        &self,
        node: &str,
        timeframe: &str,
    ) -> Result<Vec<RrdDataPoint>, ProxmoxError> {
        let url = format!(
            "{}/api2/json/nodes/{}/rrddata?timeframe={}",
            self.base_url, node, timeframe
        );
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        match resp.status() {
            reqwest::StatusCode::OK => {
                let body: serde_json::Value = resp.json().await?;
                let data = body
                    .get("data")
                    .and_then(|d| d.as_array())
                    .ok_or_else(|| ProxmoxError::Api("Missing data field".into()))?;
                let points: Vec<RrdDataPoint> = data
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                Ok(points)
            }
            _ => Err(ProxmoxError::Api(format!("HTTP {}", resp.status()))),
        }
    }

    pub async fn vm_start(&self, node: &str, vmid: u32) -> Result<String, ProxmoxError> {
        let url = format!(
            "{}/api2/json/nodes/{}/qemu/{}/status/start",
            self.base_url, node, vmid
        );
        let resp = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;
        self.handle_upid_response(resp).await
    }

    pub async fn vm_stop(&self, node: &str, vmid: u32) -> Result<String, ProxmoxError> {
        let url = format!(
            "{}/api2/json/nodes/{}/qemu/{}/status/stop",
            self.base_url, node, vmid
        );
        let resp = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;
        self.handle_upid_response(resp).await
    }

    pub async fn vm_reboot(&self, node: &str, vmid: u32) -> Result<String, ProxmoxError> {
        let url = format!(
            "{}/api2/json/nodes/{}/qemu/{}/status/reboot",
            self.base_url, node, vmid
        );
        let resp = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;
        self.handle_upid_response(resp).await
    }

    pub async fn lxc_start(&self, node: &str, vmid: u32) -> Result<String, ProxmoxError> {
        let url = format!(
            "{}/api2/json/nodes/{}/lxc/{}/status/start",
            self.base_url, node, vmid
        );
        let resp = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;
        self.handle_upid_response(resp).await
    }

    pub async fn lxc_stop(&self, node: &str, vmid: u32) -> Result<String, ProxmoxError> {
        let url = format!(
            "{}/api2/json/nodes/{}/lxc/{}/status/stop",
            self.base_url, node, vmid
        );
        let resp = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;
        self.handle_upid_response(resp).await
    }

    pub async fn lxc_reboot(&self, node: &str, vmid: u32) -> Result<String, ProxmoxError> {
        let url = format!(
            "{}/api2/json/nodes/{}/lxc/{}/status/reboot",
            self.base_url, node, vmid
        );
        let resp = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;
        self.handle_upid_response(resp).await
    }

    async fn handle_upid_response(&self, resp: reqwest::Response) -> Result<String, ProxmoxError> {
        match resp.status() {
            reqwest::StatusCode::OK => {
                let body: serde_json::Value = resp.json().await?;
                let upid = body
                    .get("data")
                    .and_then(|d| d.as_str())
                    .ok_or_else(|| ProxmoxError::Api("Missing UPID in response".into()))?;
                Ok(upid.to_string())
            }
            reqwest::StatusCode::UNAUTHORIZED => Err(ProxmoxError::Unauthorized),
            reqwest::StatusCode::FORBIDDEN => Err(ProxmoxError::Forbidden),
            _ => Err(ProxmoxError::Api(format!("HTTP {}", resp.status()))),
        }
    }

    pub async fn fetch_version(&self) -> Result<PveVersion, ProxmoxError> {
        let url = format!("{}/api2/json/version", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        match resp.status() {
            reqwest::StatusCode::OK => {
                let body: serde_json::Value = resp.json().await?;
                let data = body.get("data").ok_or_else(|| {
                    ProxmoxError::Api("Missing data field in version response".into())
                })?;
                let version: PveVersion = serde_json::from_value(data.clone())
                    .map_err(|e| ProxmoxError::Api(format!("Failed to parse version: {e}")))?;
                Ok(version)
            }
            reqwest::StatusCode::UNAUTHORIZED => Err(ProxmoxError::Unauthorized),
            reqwest::StatusCode::FORBIDDEN => Err(ProxmoxError::Forbidden),
            _ => Err(ProxmoxError::Api(format!("HTTP {}", resp.status()))),
        }
    }

    pub async fn fetch_whoami(&self) -> Result<WhoAmI, ProxmoxError> {
        let url = format!("{}/api2/json/access/whoami", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        match resp.status() {
            reqwest::StatusCode::OK => {
                let body: serde_json::Value = resp.json().await?;
                let data = body.get("data").ok_or_else(|| {
                    ProxmoxError::Api("Missing data field in whoami response".into())
                })?;
                let whoami: WhoAmI = serde_json::from_value(data.clone())
                    .map_err(|e| ProxmoxError::Api(format!("Failed to parse whoami: {e}")))?;
                Ok(whoami)
            }
            reqwest::StatusCode::UNAUTHORIZED => Err(ProxmoxError::Unauthorized),
            reqwest::StatusCode::FORBIDDEN => Err(ProxmoxError::Forbidden),
            _ => Err(ProxmoxError::Api(format!("HTTP {}", resp.status()))),
        }
    }

    pub async fn check_task_status(
        &self,
        node: &str,
        upid: &str,
    ) -> Result<TaskStatus, ProxmoxError> {
        let url = format!(
            "{}/api2/json/nodes/{}/tasks/{}/status",
            self.base_url, node, upid
        );
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        match resp.status() {
            reqwest::StatusCode::OK => {
                let body: serde_json::Value = resp.json().await?;
                let status = body
                    .get("data")
                    .and_then(|d| d.get("status"))
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| ProxmoxError::Api("Missing status field".into()))?;

                match status {
                    "OK" => Ok(TaskStatus::Completed),
                    "ERROR" => Ok(TaskStatus::Error),
                    "running" => Ok(TaskStatus::Running),
                    _ => Ok(TaskStatus::Unknown(status.to_string())),
                }
            }
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
            ProxmoxClient::new("https://pve.local:8006", "root@pam!p9s", "abc123", false).unwrap();
        assert_eq!(client.auth_header, "PVEAPIToken=root@pam!p9s=abc123");
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

    // Proxmox omits `name` for node and sdn resources; they must still parse.
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
        let resources: Vec<ClusterResource> = data
            .iter()
            .map(|v| serde_json::from_value::<ClusterResource>(v.clone()).unwrap())
            .map(|mut r| {
                if r.name.is_empty() {
                    r.name = r.node.clone().unwrap_or_else(|| r.id.clone());
                }
                r
            })
            .collect();

        assert_eq!(resources.len(), 2);
        assert_eq!(resources[0].r#type, "node");
        assert_eq!(resources[0].name, "pve-node1");
        assert_eq!(resources[1].r#type, "sdn");
        assert_eq!(resources[1].name, "sdn/zones/vlan-zone");
    }

    fn mock_unauthorized_response() -> reqwest::StatusCode {
        reqwest::StatusCode::UNAUTHORIZED
    }

    #[test]
    fn test_unauthorized_handling() {
        let status = mock_unauthorized_response();
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

    #[tokio::test]
    async fn test_connection_refused() {
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let err = client.get("http://127.0.0.1:1/").send().await.unwrap_err();

        assert!(err.is_connect() || err.is_request());

        let proxmox_err = ProxmoxError::Http(err);
        assert!(matches!(proxmox_err, ProxmoxError::Http(_)));
    }
}
