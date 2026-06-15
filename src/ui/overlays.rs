use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, view_completion};
use crate::theme::Theme;

pub fn render_filter(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let line = Line::from(vec![
        Span::styled(
            " / ",
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(app.filter.clone()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.success));

    frame.render_widget(Paragraph::new(line).block(block), area);
}

pub fn render_command(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let no_color = app.config.no_color();

    let mut spans = vec![
        Span::styled(
            " > ",
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(app.command.clone()),
    ];

    if let Some(suffix) = view_completion(&app.command) {
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
        .border_style(Style::default().fg(theme.success));

    frame.render_widget(Paragraph::new(line).block(block), area);
}
