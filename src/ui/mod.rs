mod dialog;
mod format;
mod header;
mod layout;
mod overlays;
mod table;

pub use dialog::{render_command_error, render_confirm, render_details, render_help};
pub use format::{format_disk, format_memory, view_label};
pub use layout::{centered_rect, command_error_rect};
pub use overlays::{render_command, render_filter};
use ratatui::Frame;
pub use table::render_list;

use crate::app::{App, Modal};
use crate::theme::Theme;

/// Top-level render dispatcher. Selects the appropriate view based on the
/// current modal state and delegates to the matching render function.
pub fn render(frame: &mut Frame, app: &App) {
    let theme = Theme::from_no_color(app.config.no_color);
    match &app.modal {
        Some(Modal::Help) => render_help(frame, &theme),
        Some(Modal::Filter) => render_list(frame, app, &theme),
        Some(Modal::Command) => render_list(frame, app, &theme),
        Some(Modal::CommandError(msg)) => {
            render_list(frame, app, &theme);
            render_command_error(frame, msg, &theme);
        }
        Some(Modal::Confirm(action)) => {
            render_list(frame, app, &theme);
            render_confirm(frame, action, &theme);
        }
        Some(Modal::Details) => render_details(frame, app, &theme),
        None => render_list(frame, app, &theme),
    }
}
