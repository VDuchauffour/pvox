use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use super::App;
use super::command::{extract_vmid, resolve_view, view_completion};
use super::modal::Modal;
use crate::event::{AppEvent, ConfirmAction, LifecycleAction};

impl App {
    pub fn handle_key(&mut self, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            self.quit = true;
            return;
        }

        if let Some(ref modal) = self.modal {
            match modal {
                Modal::Filter => self.handle_filter_input(key),
                Modal::Command => self.handle_command_input(key),
                Modal::CommandError(_) => self.handle_command_error_input(key),
                Modal::Confirm(action) => self.handle_confirm_input(key, action.clone(), tx),
                Modal::Help => self.handle_help_input(key),
                Modal::Details => self.handle_details_input(key),
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('?') => self.modal = Some(Modal::Help),
            KeyCode::Char('/') => self.modal = Some(Modal::Filter),
            KeyCode::Char(':') => self.modal = Some(Modal::Command),
            KeyCode::Up => self.select_prev(),
            KeyCode::Down => self.select_next(),
            KeyCode::Enter if self.current_resource().is_some() => {
                self.sparkline_data.clear();
                self.modal = Some(Modal::Details);
            }
            KeyCode::Char('s')
                if let Some(r) = self.current_resource()
                    && let (Some(node), Some(vmid)) = (r.node.clone(), extract_vmid(&r.id)) =>
            {
                let kind = r.r#type.clone();
                let _ = tx.send(AppEvent::LifecycleAction(LifecycleAction::Start {
                    node,
                    vmid,
                    kind,
                }));
                self.status_message = Some(format!("Starting {}...", r.name));
            }
            KeyCode::Char('S')
                if let Some(r) = self.current_resource()
                    && let (Some(node), Some(vmid)) = (r.node.clone(), extract_vmid(&r.id)) =>
            {
                let kind = r.r#type.clone();
                self.modal = Some(Modal::Confirm(ConfirmAction::Stop { node, vmid, kind }));
            }
            KeyCode::Char('r')
                if let Some(r) = self.current_resource()
                    && let (Some(node), Some(vmid)) = (r.node.clone(), extract_vmid(&r.id)) =>
            {
                let kind = r.r#type.clone();
                self.modal = Some(Modal::Confirm(ConfirmAction::Reboot { node, vmid, kind }));
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

    pub(crate) fn handle_filter_input(&mut self, key: KeyEvent) {
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

    pub(crate) fn handle_command_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                let input = self.command.trim().to_string();
                if let Some(view) = resolve_view(&input) {
                    self.view = view;
                    self.filter.clear();
                    self.selected_index = 0;
                    self.update_display_resources();
                    self.command.clear();
                    self.modal = None;
                } else if input.is_empty() {
                    self.command.clear();
                    self.modal = None;
                } else {
                    let bad = self.command.clone();
                    self.command.clear();
                    self.modal = Some(Modal::CommandError(bad));
                }
            }
            KeyCode::Tab => {
                if let Some(suffix) = view_completion(&self.command) {
                    self.command.push_str(suffix);
                }
            }
            KeyCode::Esc => {
                self.command.clear();
                self.modal = None;
            }
            KeyCode::Backspace => {
                self.command.pop();
            }
            KeyCode::Char(c) => {
                self.command.push(c);
            }
            _ => {}
        }
    }

    pub(crate) fn handle_command_error_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.modal = None;
            }
            _ => {}
        }
    }

    pub(crate) fn handle_confirm_input(
        &mut self,
        key: KeyEvent,
        action: ConfirmAction,
        tx: &UnboundedSender<AppEvent>,
    ) {
        match key.code {
            KeyCode::Char('y') => {
                let lifecycle_action = match action {
                    ConfirmAction::Stop { node, vmid, kind } => {
                        LifecycleAction::Stop { node, vmid, kind }
                    }
                    ConfirmAction::Reboot { node, vmid, kind } => {
                        LifecycleAction::Reboot { node, vmid, kind }
                    }
                };
                let _ = tx.send(AppEvent::LifecycleAction(lifecycle_action));
                self.modal = None;
                self.status_message = Some("Action sent...".to_string());
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.modal = None;
            }
            _ => {}
        }
    }

    pub(crate) fn handle_help_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                self.modal = None;
            }
            _ => {}
        }
    }

    pub(crate) fn handle_details_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.modal = None;
            }
            _ => {}
        }
    }
}
