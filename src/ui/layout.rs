use ratatui::layout::{Constraint, Layout, Rect};

/// Center a popup rect within the given area using percentage-based sizing.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ]);
    let popup_area = popup_layout.split(r)[1];
    let horizontal_layout = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ]);
    horizontal_layout.split(popup_area)[1]
}

/// Position a fixed-size popup rect centered within the given area.
/// Used for the command-error popup which has known dimensions.
pub fn command_error_rect(w: u16, h: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Length((r.height.saturating_sub(h)) / 2),
        Constraint::Length(h),
        Constraint::Length((r.height.saturating_sub(h)) / 2),
    ]);
    let popup_area = popup_layout.split(r)[1];
    let horizontal_layout = Layout::horizontal([
        Constraint::Length((r.width.saturating_sub(w)) / 2),
        Constraint::Min(w),
        Constraint::Length((r.width.saturating_sub(w)) / 2),
    ]);
    horizontal_layout.split(popup_area)[1]
}
