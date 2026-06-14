mod command;
mod input;
pub mod modal;
pub mod sparkline;

use std::sync::Arc;

pub use command::view_completion;
pub use modal::Modal;
pub use sparkline::SparkLineData;

use crate::api::ClusterResource;
use crate::config::Config;

pub struct App {
    pub resources: Vec<ClusterResource>,
    pub selected_index: usize,
    pub filter: String,
    pub command: String,
    pub view: String,
    pub display_resources: Vec<ClusterResource>,
    pub modal: Option<Modal>,
    pub status_message: Option<String>,
    pub connected: bool,
    pub config: Config,
    pub client: Option<Arc<crate::api::ProxmoxClient>>,
    pub pending_upids: Vec<String>,
    pub sparkline_data: SparkLineData,
    pub proxmox_version: String,
    pub proxmox_user: String,
    pub quit: bool,
    pub pending_g: bool,
}

impl App {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let client = if let (Some(host), Some(token_id), Some(token)) =
            (&config.host, &config.token_id, &config.token)
        {
            Some(Arc::new(crate::api::ProxmoxClient::new(
                host,
                token_id,
                token,
                config.insecure,
            )?))
        } else {
            None
        };

        let filter = config.filter.clone().unwrap_or_default();
        let mut app = Self {
            resources: Vec::new(),
            selected_index: 0,
            filter,
            command: String::new(),
            view: "qemu".to_string(),
            display_resources: Vec::new(),
            modal: None,
            status_message: None,
            connected: false,
            config,
            client,
            pending_upids: Vec::new(),
            sparkline_data: SparkLineData::new(),
            proxmox_version: String::new(),
            proxmox_user: String::new(),
            quit: false,
            pending_g: false,
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
        self.display_resources = self
            .resources
            .iter()
            .filter(|r| r.r#type == self.view)
            .filter(|r| {
                f.is_empty()
                    || r.name.to_lowercase().contains(&f)
                    || r.r#type.to_lowercase().contains(&f)
                    || r.node
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&f))
                        .unwrap_or(false)
            })
            .cloned()
            .collect();
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
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::command::{extract_vmid, resolve_view, view_completion};
    use super::modal::Modal;
    use super::*;
    use crate::event::{AppEvent, ConfirmAction, LifecycleAction};

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
    fn test_default_view_shows_only_vms() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        assert_eq!(app.view, "qemu");
        app.set_resources(vec![
            mock_resource("web1", "qemu", Some("pve1")),
            mock_resource("db1", "lxc", Some("pve2")),
            mock_resource("storage1", "storage", None),
        ]);
        app.set_filter("".to_string());
        assert_eq!(app.filtered_resources().len(), 1);
        assert_eq!(app.selected_resource().unwrap().name, "web1");
    }

    #[test]
    fn test_empty_filter_returns_all_in_view() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![
            mock_resource("web1", "qemu", Some("pve1")),
            mock_resource("web2", "qemu", Some("pve2")),
            mock_resource("web3", "qemu", None),
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
        assert!(
            app.filtered_resources()
                .iter()
                .all(|r| r.name.starts_with("web"))
        );
    }

    #[test]
    fn test_view_switch_to_lxc_lists_only_containers() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![
            mock_resource("vm1", "qemu", Some("pve1")),
            mock_resource("ct1", "lxc", Some("pve1")),
            mock_resource("vm2", "qemu", Some("pve2")),
        ]);
        app.view = "lxc".to_string();
        app.update_display_resources();
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
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        assert!(!app.quit);
        app.handle_key(KeyEvent::from(KeyCode::Char('q')), &tx);
        assert!(app.quit);
    }

    #[test]
    fn test_ctrl_c_quits_even_with_modal_open() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.modal = Some(Modal::Help);
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        app.handle_key(ctrl_c, &tx);
        assert!(app.quit);
    }

    #[test]
    fn test_plain_c_does_not_quit() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.handle_key(KeyEvent::from(KeyCode::Char('c')), &tx);
        assert!(!app.quit);
    }

    #[test]
    fn test_key_question_opens_help_modal() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        assert!(app.modal.is_none());
        app.handle_key(KeyEvent::from(KeyCode::Char('?')), &tx);
        assert!(matches!(app.modal, Some(Modal::Help)));
    }

    #[test]
    fn test_key_slash_opens_filter_modal() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        assert!(app.modal.is_none());
        app.handle_key(KeyEvent::from(KeyCode::Char('/')), &tx);
        assert!(matches!(app.modal, Some(Modal::Filter)));
    }

    #[test]
    fn test_key_arrows_adjust_index() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.set_resources(vec![
            mock_resource("a", "qemu", Some("pve1")),
            mock_resource("b", "qemu", Some("pve1")),
            mock_resource("c", "qemu", Some("pve1")),
        ]);
        assert_eq!(app.selected_index, 0);
        app.handle_key(KeyEvent::from(KeyCode::Down), &tx);
        assert_eq!(app.selected_index, 1);
        app.handle_key(KeyEvent::from(KeyCode::Down), &tx);
        assert_eq!(app.selected_index, 2);
        app.handle_key(KeyEvent::from(KeyCode::Down), &tx);
        assert_eq!(app.selected_index, 2);
        app.handle_key(KeyEvent::from(KeyCode::Up), &tx);
        assert_eq!(app.selected_index, 1);
        app.handle_key(KeyEvent::from(KeyCode::Up), &tx);
        assert_eq!(app.selected_index, 0);
        app.handle_key(KeyEvent::from(KeyCode::Up), &tx);
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_vim_keys_adjust_index() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.set_resources(vec![
            mock_resource("a", "qemu", Some("pve1")),
            mock_resource("b", "qemu", Some("pve1")),
            mock_resource("c", "qemu", Some("pve1")),
        ]);
        assert_eq!(app.selected_index, 0);
        app.handle_key(KeyEvent::from(KeyCode::Char('j')), &tx);
        assert_eq!(app.selected_index, 1);
        app.handle_key(KeyEvent::from(KeyCode::Char('j')), &tx);
        assert_eq!(app.selected_index, 2);
        app.handle_key(KeyEvent::from(KeyCode::Char('j')), &tx);
        assert_eq!(app.selected_index, 2);
        app.handle_key(KeyEvent::from(KeyCode::Char('k')), &tx);
        assert_eq!(app.selected_index, 1);
        app.handle_key(KeyEvent::from(KeyCode::Char('k')), &tx);
        assert_eq!(app.selected_index, 0);
        app.handle_key(KeyEvent::from(KeyCode::Char('k')), &tx);
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_gg_goes_to_first() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.set_resources(vec![
            mock_resource("a", "qemu", Some("pve1")),
            mock_resource("b", "qemu", Some("pve1")),
            mock_resource("c", "qemu", Some("pve1")),
        ]);
        app.selected_index = 2;
        app.handle_key(KeyEvent::from(KeyCode::Char('g')), &tx);
        assert!(app.pending_g);
        app.handle_key(KeyEvent::from(KeyCode::Char('g')), &tx);
        assert!(!app.pending_g);
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_g_upper_goes_to_last() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.set_resources(vec![
            mock_resource("a", "qemu", Some("pve1")),
            mock_resource("b", "qemu", Some("pve1")),
            mock_resource("c", "qemu", Some("pve1")),
        ]);
        assert_eq!(app.selected_index, 0);
        app.handle_key(KeyEvent::from(KeyCode::Char('G')), &tx);
        assert_eq!(app.selected_index, 2);
    }

    #[test]
    fn test_pending_g_cleared_by_other_keys() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.set_resources(vec![
            mock_resource("a", "qemu", Some("pve1")),
            mock_resource("b", "qemu", Some("pve1")),
            mock_resource("c", "qemu", Some("pve1")),
        ]);
        app.selected_index = 1;
        app.handle_key(KeyEvent::from(KeyCode::Char('g')), &tx);
        assert!(app.pending_g);
        app.handle_key(KeyEvent::from(KeyCode::Down), &tx);
        assert!(!app.pending_g);
        assert_eq!(app.selected_index, 2);
    }

    #[test]
    fn test_gg_then_navigation_keys() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.set_resources(vec![
            mock_resource("a", "qemu", Some("pve1")),
            mock_resource("b", "qemu", Some("pve1")),
        ]);
        app.selected_index = 1;
        app.handle_key(KeyEvent::from(KeyCode::Char('g')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('j')), &tx);
        assert!(!app.pending_g);
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn test_key_enter_opens_details_when_resource_selected() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.set_resources(vec![mock_resource("vm1", "qemu", Some("pve1"))]);
        assert!(app.modal.is_none());
        app.handle_key(KeyEvent::from(KeyCode::Enter), &tx);
        assert!(matches!(app.modal, Some(Modal::Details)));
    }

    #[test]
    fn test_key_enter_noop_when_no_resource() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        assert!(app.modal.is_none());
        app.handle_key(KeyEvent::from(KeyCode::Enter), &tx);
        assert!(app.modal.is_none());
    }

    #[test]
    fn test_filter_modal_input() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.set_resources(vec![
            mock_resource("alpha", "qemu", Some("pve1")),
            mock_resource("beta", "qemu", Some("pve1")),
        ]);

        app.handle_key(KeyEvent::from(KeyCode::Char('/')), &tx);
        assert!(matches!(app.modal, Some(Modal::Filter)));

        app.handle_key(KeyEvent::from(KeyCode::Char('a')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('l')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('p')), &tx);
        assert_eq!(app.filter, "alp");
        assert_eq!(app.filtered_resources().len(), 1);
        assert_eq!(app.filtered_resources()[0].name, "alpha");

        app.handle_key(KeyEvent::from(KeyCode::Backspace), &tx);
        assert_eq!(app.filter, "al");
        assert_eq!(app.filtered_resources().len(), 1);

        app.handle_key(KeyEvent::from(KeyCode::Char('b')), &tx);
        assert_eq!(app.filter, "alb");
        assert_eq!(app.filtered_resources().len(), 0);

        app.handle_key(KeyEvent::from(KeyCode::Esc), &tx);
        assert!(app.modal.is_none());
        assert_eq!(app.filter, "");
        assert_eq!(app.filtered_resources().len(), 2);
    }

    #[test]
    fn test_esc_resets_active_filter_in_normal_mode() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.set_resources(vec![
            mock_resource("web1", "qemu", Some("pve1")),
            mock_resource("db1", "qemu", Some("pve2")),
        ]);

        app.handle_key(KeyEvent::from(KeyCode::Char('/')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('w')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Enter), &tx);
        assert!(app.modal.is_none());
        assert_eq!(app.filter, "w");
        assert_eq!(app.filtered_resources().len(), 1);

        app.handle_key(KeyEvent::from(KeyCode::Esc), &tx);
        assert!(app.filter.is_empty());
        assert_eq!(app.filtered_resources().len(), 2);
    }

    #[test]
    fn test_esc_in_normal_mode_without_filter_is_noop() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.set_resources(vec![mock_resource("web1", "qemu", Some("pve1"))]);

        app.handle_key(KeyEvent::from(KeyCode::Esc), &tx);
        assert!(app.filter.is_empty());
        assert!(!app.quit);
        assert!(app.modal.is_none());
    }

    #[test]
    fn test_confirm_modal_keys() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.modal = Some(Modal::Confirm(ConfirmAction::Stop {
            node: "pve1".to_string(),
            vmid: 100,
            kind: "qemu".to_string(),
        }));

        app.handle_key(KeyEvent::from(KeyCode::Char('y')), &tx);
        assert!(app.modal.is_none());
        assert_eq!(app.status_message, Some("Action sent...".to_string()));
        assert!(matches!(rx.try_recv(), Ok(AppEvent::LifecycleAction(_))));

        app.modal = Some(Modal::Confirm(ConfirmAction::Reboot {
            node: "pve1".to_string(),
            vmid: 100,
            kind: "qemu".to_string(),
        }));
        app.handle_key(KeyEvent::from(KeyCode::Char('n')), &tx);
        assert!(app.modal.is_none());

        app.modal = Some(Modal::Confirm(ConfirmAction::Reboot {
            node: "pve1".to_string(),
            vmid: 100,
            kind: "lxc".to_string(),
        }));
        app.handle_key(KeyEvent::from(KeyCode::Esc), &tx);
        assert!(app.modal.is_none());
    }

    #[test]
    fn test_help_modal_close_keys() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.modal = Some(Modal::Help);

        app.handle_key(KeyEvent::from(KeyCode::Char('?')), &tx);
        assert!(app.modal.is_none());

        app.modal = Some(Modal::Help);
        app.handle_key(KeyEvent::from(KeyCode::Esc), &tx);
        assert!(app.modal.is_none());

        app.modal = Some(Modal::Help);
        app.handle_key(KeyEvent::from(KeyCode::Char('q')), &tx);
        assert!(app.modal.is_none());
    }

    #[test]
    fn test_details_modal_close_keys() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.modal = Some(Modal::Details);

        app.handle_key(KeyEvent::from(KeyCode::Esc), &tx);
        assert!(app.modal.is_none());

        app.modal = Some(Modal::Details);
        app.handle_key(KeyEvent::from(KeyCode::Char('q')), &tx);
        assert!(app.modal.is_none());
    }

    #[test]
    fn test_key_s_sends_lifecycle_start() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        let mut resources = vec![mock_resource("vm1", "qemu", Some("pve1"))];
        resources[0].id = "qemu/100".to_string();
        app.set_resources(resources);
        app.handle_key(KeyEvent::from(KeyCode::Char('s')), &tx);
        assert_eq!(app.status_message, Some("Starting vm1...".to_string()));
        assert!(
            matches!(rx.try_recv(), Ok(AppEvent::LifecycleAction(LifecycleAction::Start { node, vmid, kind })) if node == "pve1" && vmid == 100 && kind == "qemu")
        );
    }

    #[test]
    fn test_key_upper_s_opens_stop_confirm() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.set_resources(vec![mock_resource("100", "qemu", Some("pve1"))]);
        app.resources[0].id = "qemu/100".to_string();
        app.handle_key(KeyEvent::from(KeyCode::Char('S')), &tx);
        match app.modal {
            Some(Modal::Confirm(ConfirmAction::Stop { node, vmid, kind })) => {
                assert_eq!(node, "pve1");
                assert_eq!(vmid, 100);
                assert_eq!(kind, "qemu");
            }
            _ => panic!("Expected Stop confirm modal"),
        }
    }

    #[test]
    fn test_key_r_opens_reboot_confirm() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.view = "lxc".to_string();
        app.set_resources(vec![mock_resource("200", "lxc", Some("pve2"))]);
        app.resources[0].id = "lxc/200".to_string();
        app.handle_key(KeyEvent::from(KeyCode::Char('r')), &tx);
        match app.modal {
            Some(Modal::Confirm(ConfirmAction::Reboot { node, vmid, kind })) => {
                assert_eq!(node, "pve2");
                assert_eq!(vmid, 200);
                assert_eq!(kind, "lxc");
            }
            _ => panic!("Expected Reboot confirm modal"),
        }
    }

    #[test]
    fn test_extract_vmid_parses_id() {
        assert_eq!(extract_vmid("qemu/100"), Some(100));
        assert_eq!(extract_vmid("lxc/200"), Some(200));
        assert_eq!(extract_vmid("node/pve"), None);
        assert_eq!(extract_vmid("invalid"), None);
    }

    fn key(c: char) -> KeyEvent {
        KeyEvent::from(KeyCode::Char(c))
    }

    fn key_code(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    #[test]
    fn test_confirm_yes() {
        let mut app = App::new(mock_config()).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.modal = Some(Modal::Confirm(ConfirmAction::Stop {
            node: "pve1".to_string(),
            vmid: 100,
            kind: "qemu".to_string(),
        }));

        app.handle_confirm_input(
            key('y'),
            ConfirmAction::Stop {
                node: "pve1".to_string(),
                vmid: 100,
                kind: "qemu".to_string(),
            },
            &tx,
        );

        assert!(app.modal.is_none());
        assert!(app.status_message.is_some());
    }

    #[test]
    fn test_confirm_no() {
        let mut app = App::new(mock_config()).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.modal = Some(Modal::Confirm(ConfirmAction::Reboot {
            node: "pve1".to_string(),
            vmid: 100,
            kind: "qemu".to_string(),
        }));

        app.handle_confirm_input(
            key('n'),
            ConfirmAction::Reboot {
                node: "pve1".to_string(),
                vmid: 100,
                kind: "qemu".to_string(),
            },
            &tx,
        );

        assert!(app.modal.is_none());
        assert!(app.status_message.is_none());
    }

    #[test]
    fn test_filter_input_capture() {
        let mut app = App::new(mock_config()).unwrap();
        app.modal = Some(Modal::Filter);

        app.handle_filter_input(key('w'));
        app.handle_filter_input(key('e'));
        app.handle_filter_input(key('b'));

        assert_eq!(app.filter, "web");
    }

    #[test]
    fn test_filter_backspace() {
        let mut app = App::new(mock_config()).unwrap();
        app.modal = Some(Modal::Filter);
        app.filter = "web".to_string();

        app.handle_filter_input(key_code(KeyCode::Backspace));

        assert_eq!(app.filter, "we");
    }

    #[test]
    fn test_help_modal_doesnt_block_arrows() {
        let mut app = App::new(mock_config()).unwrap();
        app.set_resources(vec![
            mock_resource("a", "qemu", Some("pve1")),
            mock_resource("b", "qemu", Some("pve1")),
        ]);
        app.selected_index = 1;

        app.modal = Some(Modal::Help);

        app.handle_help_input(key_code(KeyCode::Up));

        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn test_modal_transitions() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();

        app.modal = Some(Modal::Help);
        assert!(matches!(app.modal, Some(Modal::Help)));

        app.modal = None;
        assert!(app.modal.is_none());

        app.modal = Some(Modal::Filter);
        assert!(matches!(app.modal, Some(Modal::Filter)));

        app.modal = Some(Modal::Confirm(ConfirmAction::Stop {
            node: "pve1".to_string(),
            vmid: 100,
            kind: "qemu".to_string(),
        }));
        assert!(matches!(app.modal, Some(Modal::Confirm(_))));
    }

    #[test]
    fn test_keyboard_dispatch_with_modal() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();

        app.modal = Some(Modal::Help);
        app.quit = false;
        app.handle_key(KeyEvent::from(KeyCode::Char('q')), &tx);
        assert!(app.modal.is_none());
        assert!(!app.quit);

        app.modal = Some(Modal::Details);
        app.quit = false;
        app.handle_key(KeyEvent::from(KeyCode::Char('q')), &tx);
        assert!(app.modal.is_none());
        assert!(!app.quit);
    }

    #[test]
    fn test_selected_resource_bounds() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![
            mock_resource("vm1", "qemu", Some("pve1")),
            mock_resource("vm2", "qemu", Some("pve1")),
        ]);

        app.selected_index = 0;
        assert!(app.current_resource().is_some());
        assert_eq!(app.current_resource().unwrap().name, "vm1");

        app.selected_index = 1;
        assert!(app.current_resource().is_some());
        assert_eq!(app.current_resource().unwrap().name, "vm2");

        app.selected_index = 999;
        assert!(app.current_resource().is_none());
    }

    #[test]
    fn test_filter_state_transition() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.set_resources(vec![
            mock_resource("web1", "qemu", Some("pve1")),
            mock_resource("db1", "qemu", Some("pve2")),
            mock_resource("web2", "qemu", Some("pve2")),
        ]);

        app.set_filter("web".to_string());
        assert_eq!(app.filter, "web");
        assert_eq!(app.filtered_resources().len(), 2);
        assert!(
            app.filtered_resources()
                .iter()
                .all(|r| r.name.starts_with("web"))
        );

        app.set_filter("".to_string());
        assert!(app.filter.is_empty());
        assert_eq!(app.filtered_resources().len(), 3);
    }

    #[test]
    fn test_command_opens_modal() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        assert!(app.modal.is_none());
        app.handle_key(KeyEvent::from(KeyCode::Char(':')), &tx);
        assert!(matches!(app.modal, Some(Modal::Command)));
    }

    #[test]
    fn test_command_input_typing() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.modal = Some(Modal::Command);
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();

        app.handle_key(KeyEvent::from(KeyCode::Char('n')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('o')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('e')), &tx);
        assert_eq!(app.command, "node");
    }

    #[test]
    fn test_command_enter_switches_view() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.set_resources(vec![
            mock_resource("pve1", "node", Some("pve1")),
            mock_resource("vm1", "qemu", Some("pve1")),
        ]);

        app.handle_key(KeyEvent::from(KeyCode::Char(':')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('n')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('o')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('e')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Enter), &tx);

        assert!(app.modal.is_none());
        assert_eq!(app.view, "node");
        assert_eq!(app.filtered_resources().len(), 1);
        assert_eq!(app.filtered_resources()[0].name, "pve1");
        assert!(app.command.is_empty());
    }

    #[test]
    fn test_command_escape_cancels() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.modal = Some(Modal::Command);
        app.command = "node".to_string();

        app.handle_key(KeyEvent::from(KeyCode::Esc), &tx);

        assert!(app.modal.is_none());
        assert_eq!(app.view, "qemu");
        assert!(app.command.is_empty());
    }

    #[test]
    fn test_command_backspace() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        app.modal = Some(Modal::Command);
        app.command = "node".to_string();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();

        app.handle_key(KeyEvent::from(KeyCode::Backspace), &tx);
        assert_eq!(app.command, "nod");
    }

    #[test]
    fn test_resolve_view_aliases() {
        assert_eq!(resolve_view("vm"), Some("qemu".to_string()));
        assert_eq!(resolve_view("vms"), Some("qemu".to_string()));
        assert_eq!(resolve_view("qemu"), Some("qemu".to_string()));
        assert_eq!(resolve_view("VM"), Some("qemu".to_string()));
        assert_eq!(resolve_view("node"), Some("node".to_string()));
        assert_eq!(resolve_view("nodes"), Some("node".to_string()));
        assert_eq!(resolve_view("ct"), Some("lxc".to_string()));
        assert_eq!(resolve_view("container"), Some("lxc".to_string()));
        assert_eq!(resolve_view("containers"), Some("lxc".to_string()));
        assert_eq!(resolve_view("lxc"), Some("lxc".to_string()));
        assert_eq!(resolve_view("storage"), Some("storage".to_string()));
        assert_eq!(resolve_view("storages"), Some("storage".to_string()));
        assert_eq!(resolve_view(""), None);
        assert_eq!(resolve_view("sdn"), None);
    }

    #[test]
    fn test_command_error_on_invalid_input() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.set_resources(vec![mock_resource("vm1", "qemu", Some("pve1"))]);

        app.handle_key(KeyEvent::from(KeyCode::Char(':')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('f')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('o')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Char('o')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Enter), &tx);

        assert!(matches!(app.modal, Some(Modal::CommandError(ref msg)) if msg == "foo"));
        assert!(app.command.is_empty());
        assert_eq!(app.view, "qemu");
    }

    #[test]
    fn test_command_error_dismiss_on_escape() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.modal = Some(Modal::CommandError("xyz".to_string()));

        app.handle_key(KeyEvent::from(KeyCode::Esc), &tx);
        assert!(app.modal.is_none());
    }

    #[test]
    fn test_command_error_dismiss_on_enter() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.modal = Some(Modal::CommandError("xyz".to_string()));

        app.handle_key(KeyEvent::from(KeyCode::Enter), &tx);
        assert!(app.modal.is_none());
    }

    #[test]
    fn test_command_empty_enter_closes_modal() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();

        app.handle_key(KeyEvent::from(KeyCode::Char(':')), &tx);
        app.handle_key(KeyEvent::from(KeyCode::Enter), &tx);
        assert!(app.modal.is_none());
        assert_eq!(app.view, "qemu");
    }

    #[test]
    fn test_view_completion_partial() {
        assert_eq!(view_completion("n"), Some("ode"));
        assert_eq!(view_completion("no"), Some("de"));
        assert_eq!(view_completion("nod"), Some("e"));
        assert_eq!(view_completion("v"), Some("m"));
        assert_eq!(view_completion("q"), Some("emu"));
        assert_eq!(view_completion("ct"), Some(""));
        assert_eq!(view_completion("l"), Some("xc"));
        assert_eq!(view_completion("st"), Some("orage"));
    }

    #[test]
    fn test_view_completion_full_match() {
        assert_eq!(view_completion("node"), Some(""));
        assert_eq!(view_completion("qemu"), Some(""));
        assert_eq!(view_completion("vm"), Some(""));
    }

    #[test]
    fn test_view_completion_no_match() {
        assert_eq!(view_completion("xyz"), None);
        assert_eq!(view_completion(""), None);
    }

    #[test]
    fn test_tab_accepts_completion() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.modal = Some(Modal::Command);
        app.command = "no".to_string();

        app.handle_key(KeyEvent::from(KeyCode::Tab), &tx);
        assert_eq!(app.command, "node");
    }

    #[test]
    fn test_tab_no_completion_is_noop() {
        let config = mock_config();
        let mut app = App::new(config).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
        app.modal = Some(Modal::Command);
        app.command = "xyz".to_string();

        app.handle_key(KeyEvent::from(KeyCode::Tab), &tx);
        assert_eq!(app.command, "xyz");
    }
}
