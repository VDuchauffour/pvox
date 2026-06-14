use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::theme::Theme;

/// Render the filter input bar at the bottom of the list view.
pub fn render_filter(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let line = Line::from(vec![
        Span::styled(" / ", theme.prompt()),
        Span::raw(app.filter.clone()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent));

    frame.render_widget(Paragraph::new(line).block(block), area);
}

/// Render the command input bar at the bottom of the list view, with tab-completion hint.
pub fn render_command(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let no_color = app.config.no_color;

    let mut spans = vec![
        Span::styled(" > ", theme.prompt()),
        Span::raw(app.command.clone()),
    ];

    if let Some(suffix) = App::view_completion(&app.command) {
        let completion_style = if no_color {
            theme.completion_no_color()
        } else {
            theme.completion()
        };
        spans.push(Span::styled(suffix.to_string(), completion_style));
    }

    let line = Line::from(spans);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent));

    frame.render_widget(Paragraph::new(line).block(block), area);
}
