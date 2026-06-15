mod dialog;
mod format;
mod header;
mod help;
mod layout;
mod overlays;
mod table;

pub use dialog::{render_command_error, render_confirm, render_details};
pub use format::{format_disk, format_memory, view_label};
pub use help::render_help;
pub use layout::{centered_rect, command_error_rect};
pub use overlays::{render_command, render_filter};
use ratatui::Frame;
pub use table::render_list;

use crate::app::{App, Modal};
use crate::theme::Theme;

/// Trait for rendering modal overlays. Each modal variant decides how it
/// renders itself and whether the underlying list view should be shown first.
pub trait ModalRenderer {
    fn render(&self, frame: &mut Frame, app: &App, theme: &Theme);
}

impl ModalRenderer for Modal {
    fn render(&self, frame: &mut Frame, app: &App, theme: &Theme) {
        match self {
            Modal::Help => render_help(frame, app, theme),
            Modal::Filter => render_list(frame, app, theme),
            Modal::Command => render_list(frame, app, theme),
            Modal::CommandError(msg) => {
                render_list(frame, app, theme);
                render_command_error(frame, msg, theme);
            }
            Modal::Confirm(action) => {
                render_list(frame, app, theme);
                render_confirm(frame, action, theme);
            }
            Modal::Details => render_details(frame, app, theme),
        }
    }
}

/// Top-level render dispatcher. Selects the appropriate view based on the
/// current modal state and delegates to the matching render function.
pub fn render(frame: &mut Frame, app: &App) {
    let theme = Theme::from_no_color(app.config.no_color());
    match &app.modal {
        Some(modal) => modal.render(frame, app, &theme),
        None => render_list(frame, app, &theme),
    }
}
