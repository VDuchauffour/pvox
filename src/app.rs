use std::sync::Arc;

use crate::client::{ClusterResource, ProxmoxClient};
use crate::config::Config;
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone)]
pub enum Modal {
    Help,
    Filter,
    Confirm(ConfirmAction),
    Details,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    Stop { node: String, vmid: u32 },
    Reboot { node: String, vmid: u32 },
}

pub struct SparklineData {
    pub cpu_history: Vec<u64>, // 60 points
    pub mem_history: Vec<u64>,
}

impl SparklineData {
    pub fn new() -> Self {
        Self {
            cpu_history: Vec::with_capacity(60),
            mem_history: Vec::with_capacity(60),
        }
    }

    pub fn push_cpu(&mut self, value: u64) {
        if self.cpu_history.len() >= 60 {
            self.cpu_history.remove(0);
        }
        self.cpu_history.push(value);
    }

    pub fn push_mem(&mut self, value: u64) {
        if self.mem_history.len() >= 60 {
            self.mem_history.remove(0);
        }
        self.mem_history.push(value);
    }

    pub fn clear(&mut self) {
        self.cpu_history.clear();
        self.mem_history.clear();
    }
}

pub struct App {
    pub resources: Vec<ClusterResource>,
    pub selected_index: usize,
    pub filter: String,
    pub display_resources: Vec<ClusterResource>,
    pub modal: Option<Modal>,
    pub status_message: Option<String>,
    pub connected: bool,
    pub config: Config,
    pub client: Option<Arc<ProxmoxClient>>,
    pub pending_upids: Vec<String>,
    pub sparkline_data: SparklineData,
    pub quit: bool,
}

impl App {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let client = if let (Some(host), Some(token_id), Some(token)) =
            (&config.host, &config.token_id, &config.token)
        {
            Some(ProxmoxClient::new(host, token_id, token, config.insecure)?)
        } else {
            None
        };

        let filter = config.filter.clone().unwrap_or_default();
        let mut app = Self {
            resources: Vec::new(),
            selected_index: 0,
            filter,
            display_resources: Vec::new(),
            modal: None,
            status_message: None,
            connected: false,
            config,
            client: client.map(Arc::new),
            pending_upids: Vec::new(),
            sparkline_data: SparklineData::new(),
            quit: false,
        };
        app.update_display_resources();
        Ok(app)
    }

    pub fn filtered_resources(&self) -> &[ClusterResource] {
        &self.display_resources
    }

    pub fn selected_resource(&self) -> Option<&ClusterResource> {
        self.display_resources.get(self.selected_index)
    }

    pub fn current_resource(&self) -> Option<&ClusterResource> {
        self.selected_resource()
    }

    pub fn complete_upid(&mut self, upid: &str) {
        self.pending_upids.retain(|u| u != upid);
        self.status_message = Some(format!("Task completed: {}", upid));
    }

    pub fn update_display_resources(&mut self) {
        let f = self.filter.to_lowercase();
        if f.is_empty() {
            self.display_resources = self.resources.clone();
        } else {
            self.display_resources = self
                .resources
                .iter()
                .filter(|r| {
                    r.name.to_lowercase().contains(&f)
                        || r.r#type.to_lowercase().contains(&f)
                        || r.node
                            .as_ref()
                            .map(|n| n.to_lowercase().contains(&f))
                            .unwrap_or(false)
                })
                .cloned()
                .collect();
        }
        self.selected_index = self
            .selected_index
            .min(self.display_resources.len().saturating_sub(1));
    }

    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
        self.selected_index = 0;
        self.update_display_resources();
    }

    pub fn set_resources(&mut self, resources: Vec<ClusterResource>) {
        self.resources = resources;
        self.update_display_resources();
        if self.resources.is_empty() {
            self.selected_index = 0;
        } else {
            self.selected_index = self
                .selected_index
                .min(self.display_resources.len().saturating_sub(1));
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if let Some(ref modal) = self.modal {
            match modal {
                Modal::Filter => self.handle_filter_input(key),
                Modal::Confirm(action) => self.handle_confirm_input(key, action.clone()),
                Modal::Help => self.handle_help_input(key),
                Modal::Details => self.handle_details_input(key),
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('?') => self.modal = Some(Modal::Help),
            KeyCode::Char('/') => self.modal = Some(Modal::Filter),
            KeyCode::Up => self.select_prev(),
            KeyCode::Down => self.select_next(),
            KeyCode::Enter => {
                if self.current_resource().is_some() {
                    self.sparkline_data.clear();
                    self.modal = Some(Modal::Details);
                }
            }
            KeyCode::Char('s') => {
                if let Some(r) = self.current_resource() {
                    self.status_message = Some(format!("Starting {}...", r.name));
                }
            }
            KeyCode::Char('S') => {
                if let Some(r) = self.current_resource() {
                    if let Some(node) = r.node.clone() {
                        if let Some(vmid) = Self::extract_vmid(&r.id) {
                            self.modal = Some(Modal::Confirm(ConfirmAction::Stop { node, vmid }));
                        }
                    }
                }
            }
            KeyCode::Char('r') => {
                if let Some(r) = self.current_resource() {
                    if let Some(node) = r.node.clone() {
                        if let Some(vmid) = Self::extract_vmid(&r.id) {
                            self.modal = Some(Modal::Confirm(ConfirmAction::Reboot { node, vmid }));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn select_next(&mut self) {
        if !self.display_resources.is_empty()
            && self.selected_index < self.display_resources.len() - 1
        {
            self.selected_index += 1;
        }
    }

    fn handle_filter_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => self.modal = None,
            KeyCode::Esc => {
                self.filter.clear();
                self.update_display_resources();
                self.modal = None;
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.update_display_resources();
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.update_display_resources();
            }
            _ => {}
        }
    }

    fn handle_confirm_input(&mut self, key: KeyEvent, _action: ConfirmAction) {
        match key.code {
            KeyCode::Char('y') => {
                self.status_message = Some("Confirming action...".to_string());
                self.modal = None;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.modal = None;
            }
            _ => {}
        }
    }

    fn handle_help_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                self.modal = None;
            }
            _ => {}
        }
    }

    fn handle_details_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.modal = None;
            }
            _ => {}
        }
    }

    fn extract_vmid(id: &str) -> Option<u32> {
        id.split('/').nth(1)?.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_resource(name: &str, rtype: &str, node: Option<&str>) -> ClusterResource {
        ClusterResource {
            id: format!("{}/{}", rtype, name),
            r#type: rtype.to_string(),
            name: name.to_string(),
            node: node.map(|n| n.to_string()),
            status: "running".to_string(),
            cpu: None,
            maxcpu: None,
            mem: None,
            maxmem: None,
            disk: None,
            maxdisk: None,
            uptime: None,
        }
    }

    fn mock_config() -> Config {
        Config {
            host: None,
            token_id: None,
            token: None,
            insecure: false,
            refresh_interval: 5,
            filter: None,
            no_color: false,
            config: None,
        }
    }

    #[test]
    fn test_empty_filter_returns_all() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![
            mock_resource("web1", "qemu", Some("pve1")),
            mock_resource("db1", "lxc", Some("pve2")),
            mock_resource("storage1", "storage", None),
        ]);
        app.set_filter("".to_string());
        assert_eq!(app.filtered_resources().len(), 3);
        assert_eq!(app.selected_resource().unwrap().name, "web1");
    }

    #[test]
    fn test_filter_subset_by_name() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![
            mock_resource("web1", "qemu", Some("pve1")),
            mock_resource("web2", "qemu", Some("pve1")),
            mock_resource("db1", "lxc", Some("pve2")),
            mock_resource("cache1", "qemu", Some("pve2")),
            mock_resource("storage1", "storage", None),
        ]);
        app.set_filter("web".to_string());
        assert_eq!(app.filtered_resources().len(), 2);
        assert!(app
            .filtered_resources()
            .iter()
            .all(|r| r.name.starts_with("web")));
    }

    #[test]
    fn test_filter_subset_by_type() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![
            mock_resource("vm1", "qemu", Some("pve1")),
            mock_resource("ct1", "lxc", Some("pve1")),
            mock_resource("vm2", "qemu", Some("pve2")),
        ]);
        app.set_filter("lxc".to_string());
        assert_eq!(app.filtered_resources().len(), 1);
        assert_eq!(app.filtered_resources()[0].name, "ct1");
    }

    #[test]
    fn test_filter_subset_by_node() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![
            mock_resource("vm1", "qemu", Some("pve1")),
            mock_resource("vm2", "qemu", Some("pve2")),
            mock_resource("vm3", "qemu", Some("pve1")),
        ]);
        app.set_filter("pve2".to_string());
        assert_eq!(app.filtered_resources().len(), 1);
        assert_eq!(app.filtered_resources()[0].name, "vm2");
    }

    #[test]
    fn test_filter_case_insensitive() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![mock_resource("WebServer", "qemu", Some("PVE1"))]);
        app.set_filter("web".to_string());
        assert_eq!(app.filtered_resources().len(), 1);
        app.set_filter("PVE".to_string());
        assert_eq!(app.filtered_resources().len(), 1);
    }

    #[test]
    fn test_selected_bounds_after_filter() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![
            mock_resource("alpha", "qemu", Some("pve1")),
            mock_resource("beta", "qemu", Some("pve1")),
            mock_resource("gamma", "qemu", Some("pve1")),
        ]);
        app.selected_index = 2;
        app.set_filter("alpha".to_string());
        assert_eq!(app.filtered_resources().len(), 1);
        assert_eq!(app.selected_index, 0);
        assert_eq!(app.selected_resource().unwrap().name, "alpha");
    }

    #[test]
    fn test_selected_resource_none_when_empty() {
        let config = mock_config();
        let app = App::new(config).unwrap();
        assert!(app.selected_resource().is_none());
    }

    #[test]
    fn test_filter_no_match_returns_empty() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![mock_resource("vm1", "qemu", Some("pve1"))]);
        app.set_filter("nonexistent".to_string());
        assert!(app.filtered_resources().is_empty());
        assert!(app.selected_resource().is_none());
    }

    #[test]
    fn test_filter_vm_matches_type_qemu_and_name_vm_100() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![
            mock_resource("vm-100", "qemu", Some("pve1")),
            mock_resource("web1", "qemu", Some("pve1")),
            mock_resource("db1", "lxc", Some("pve2")),
        ]);
        app.set_filter("vm".to_string());
        assert_eq!(app.filtered_resources().len(), 1);
        assert_eq!(app.filtered_resources()[0].name, "vm-100");
    }

    #[test]
    fn test_key_q_sets_quit() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        assert!(!app.quit);
        app.handle_key(KeyEvent::from(KeyCode::Char('q')));
        assert!(app.quit);
    }

    #[test]
    fn test_key_question_opens_help_modal() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        assert!(app.modal.is_none());
        app.handle_key(KeyEvent::from(KeyCode::Char('?')));
        assert!(matches!(app.modal, Some(Modal::Help)));
    }

    #[test]
    fn test_key_slash_opens_filter_modal() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        assert!(app.modal.is_none());
        app.handle_key(KeyEvent::from(KeyCode::Char('/')));
        assert!(matches!(app.modal, Some(Modal::Filter)));
    }

    #[test]
    fn test_key_arrows_adjust_index() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![
            mock_resource("a", "qemu", Some("pve1")),
            mock_resource("b", "qemu", Some("pve1")),
            mock_resource("c", "qemu", Some("pve1")),
        ]);
        assert_eq!(app.selected_index, 0);
        app.handle_key(KeyEvent::from(KeyCode::Down));
        assert_eq!(app.selected_index, 1);
        app.handle_key(KeyEvent::from(KeyCode::Down));
        assert_eq!(app.selected_index, 2);
        app.handle_key(KeyEvent::from(KeyCode::Down));
        assert_eq!(app.selected_index, 2);
        app.handle_key(KeyEvent::from(KeyCode::Up));
        assert_eq!(app.selected_index, 1);
        app.handle_key(KeyEvent::from(KeyCode::Up));
        assert_eq!(app.selected_index, 0);
        app.handle_key(KeyEvent::from(KeyCode::Up));
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_key_enter_opens_details_when_resource_selected() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![mock_resource("vm1", "qemu", Some("pve1"))]);
        assert!(app.modal.is_none());
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert!(matches!(app.modal, Some(Modal::Details)));
    }

    #[test]
    fn test_key_enter_noop_when_no_resource() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        assert!(app.modal.is_none());
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert!(app.modal.is_none());
    }

    #[test]
    fn test_filter_modal_input() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![
            mock_resource("alpha", "qemu", Some("pve1")),
            mock_resource("beta", "qemu", Some("pve1")),
        ]);

        app.handle_key(KeyEvent::from(KeyCode::Char('/')));
        assert!(matches!(app.modal, Some(Modal::Filter)));

        app.handle_key(KeyEvent::from(KeyCode::Char('a')));
        app.handle_key(KeyEvent::from(KeyCode::Char('l')));
        app.handle_key(KeyEvent::from(KeyCode::Char('p')));
        assert_eq!(app.filter, "alp");
        assert_eq!(app.filtered_resources().len(), 1);
        assert_eq!(app.filtered_resources()[0].name, "alpha");

        app.handle_key(KeyEvent::from(KeyCode::Backspace));
        assert_eq!(app.filter, "al");
        assert_eq!(app.filtered_resources().len(), 1);

        app.handle_key(KeyEvent::from(KeyCode::Char('b')));
        assert_eq!(app.filter, "alb");
        assert_eq!(app.filtered_resources().len(), 0);

        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert!(app.modal.is_none());
        assert_eq!(app.filter, "");
        assert_eq!(app.filtered_resources().len(), 2);
    }

    #[test]
    fn test_confirm_modal_keys() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.modal = Some(Modal::Confirm(ConfirmAction::Stop {
            node: "pve1".to_string(),
            vmid: 100,
        }));

        app.handle_key(KeyEvent::from(KeyCode::Char('y')));
        assert!(app.modal.is_none());
        assert!(app.status_message.is_some());

        app.modal = Some(Modal::Confirm(ConfirmAction::Reboot {
            node: "pve1".to_string(),
            vmid: 100,
        }));
        app.handle_key(KeyEvent::from(KeyCode::Char('n')));
        assert!(app.modal.is_none());

        app.modal = Some(Modal::Confirm(ConfirmAction::Reboot {
            node: "pve1".to_string(),
            vmid: 100,
        }));
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert!(app.modal.is_none());
    }

    #[test]
    fn test_help_modal_close_keys() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.modal = Some(Modal::Help);

        app.handle_key(KeyEvent::from(KeyCode::Char('?')));
        assert!(app.modal.is_none());

        app.modal = Some(Modal::Help);
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert!(app.modal.is_none());

        app.modal = Some(Modal::Help);
        app.handle_key(KeyEvent::from(KeyCode::Char('q')));
        assert!(app.modal.is_none());
    }

    #[test]
    fn test_details_modal_close_keys() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.modal = Some(Modal::Details);

        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert!(app.modal.is_none());

        app.modal = Some(Modal::Details);
        app.handle_key(KeyEvent::from(KeyCode::Char('q')));
        assert!(app.modal.is_none());
    }

    #[test]
    fn test_key_s_sets_status_message() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![mock_resource("vm1", "qemu", Some("pve1"))]);
        app.handle_key(KeyEvent::from(KeyCode::Char('s')));
        assert_eq!(app.status_message, Some("Starting vm1...".to_string()));
    }

    #[test]
    fn test_key_upper_s_opens_stop_confirm() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![mock_resource("100", "qemu", Some("pve1"))]);
        app.resources[0].id = "qemu/100".to_string();
        app.handle_key(KeyEvent::from(KeyCode::Char('S')));
        match app.modal {
            Some(Modal::Confirm(ConfirmAction::Stop { node, vmid })) => {
                assert_eq!(node, "pve1");
                assert_eq!(vmid, 100);
            }
            _ => panic!("Expected Stop confirm modal"),
        }
    }

    #[test]
    fn test_key_r_opens_reboot_confirm() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![mock_resource("200", "lxc", Some("pve2"))]);
        app.resources[0].id = "lxc/200".to_string();
        app.handle_key(KeyEvent::from(KeyCode::Char('r')));
        match app.modal {
            Some(Modal::Confirm(ConfirmAction::Reboot { node, vmid })) => {
                assert_eq!(node, "pve2");
                assert_eq!(vmid, 200);
            }
            _ => panic!("Expected Reboot confirm modal"),
        }
    }

    #[test]
    fn test_extract_vmid_parses_id() {
        assert_eq!(App::extract_vmid("qemu/100"), Some(100));
        assert_eq!(App::extract_vmid("lxc/200"), Some(200));
        assert_eq!(App::extract_vmid("node/pve"), None);
        assert_eq!(App::extract_vmid("invalid"), None);
    }
}
