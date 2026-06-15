use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Sparkline, Wrap},
};

use super::header::render_header;
use super::layout::command_error_rect;
use super::table::{HEADER_HEIGHT, render_status_bar};
use crate::app::App;
use crate::event::ConfirmAction;
use crate::theme::Theme;

/// Render a confirmation dialog for stop/reboot actions.
pub fn render_confirm(frame: &mut Frame, action: &ConfirmAction, theme: &Theme) {
    let msg = match action {
        ConfirmAction::Stop { node, vmid, .. } => format!("Stop {} on {}? (y/n)", vmid, node),
        ConfirmAction::Reboot { node, vmid, .. } => format!("Reboot {} on {}? (y/n)", vmid, node),
    };
    let btn = "[ Confirm ]";
    let max_w = msg.len().max(btn.len()) as u16 + 4;
    let h: u16 = 6;
    let area = command_error_rect(max_w, h, frame.area());
    frame.render_widget(Clear, area);
    let text = Text::from(vec![
        Line::from(""),
        Line::from(msg.clone()),
        Line::from(""),
        Line::from(Span::styled(btn, theme.accent_bg_bold())),
    ]);
    frame.render_widget(
        Paragraph::new(text).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent))
                .title(Span::styled(" Confirm ", theme.accent_bold()))
                .title_alignment(Alignment::Center),
        ),
        area,
    );
}

/// Render a dismissible error popup for unknown commands.
pub fn render_command_error(frame: &mut Frame, command: &str, theme: &Theme) {
    let line1 = format!("`{}` command not found", command);
    let btn = "[ Dismiss ]";
    let max_w = line1.len().max(btn.len()) as u16 + 4;
    let h: u16 = 6;
    let area = command_error_rect(max_w, h, frame.area());
    frame.render_widget(Clear, area);
    let text = Text::from(vec![
        Line::from(""),
        Line::from(line1.clone()),
        Line::from(""),
        Line::from(Span::styled(btn, theme.accent_bg_bold())),
    ]);
    frame.render_widget(
        Paragraph::new(text).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent))
                .title(Span::styled(" Error ", theme.accent_bold()))
                .title_alignment(Alignment::Center),
        ),
        area,
    );
}

/// Render the resource details full-frame view with sparkline history.
pub fn render_details(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = frame.area();
    let body_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height.saturating_sub(1),
    };

    let [header_area, main_area] =
        Layout::vertical([Constraint::Length(HEADER_HEIGHT), Constraint::Min(0)]).areas(body_area);

    render_header(frame, app, header_area, theme);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(" Resource Details ", theme.accent_bold()))
        .title_alignment(Alignment::Center)
        .padding(ratatui::widgets::Padding::new(1, 1, 0, 0));

    let inner = block.inner(main_area);
    frame.render_widget(block, main_area);

    let chunks = Layout::vertical([Constraint::Min(8), Constraint::Length(8)]).split(inner);

    let content = app
        .current_resource()
        .map(|r| r.format_details())
        .unwrap_or_else(|| "No resource selected".to_string());

    let paragraph = Paragraph::new(content).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, chunks[0]);

    let sparkline_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(" History ", theme.accent_bold()))
        .title_alignment(Alignment::Center)
        .padding(ratatui::widgets::Padding::new(1, 1, 0, 0));

    let sparkline_inner = sparkline_block.inner(chunks[1]);
    frame.render_widget(sparkline_block, chunks[1]);

    let sparkline_chunks = Layout::vertical([Constraint::Length(1), Constraint::Length(1)])
        .margin(1)
        .split(sparkline_inner);

    if !app.sparkline_data.cpu_history.is_empty() {
        let cpu_sparkline = Sparkline::default()
            .data(&app.sparkline_data.cpu_history)
            .style(theme.sparkline_cpu());
        frame.render_widget(cpu_sparkline, sparkline_chunks[0]);
        let mem_sparkline = Sparkline::default()
            .data(&app.sparkline_data.mem_history)
            .style(theme.sparkline_mem());
        frame.render_widget(mem_sparkline, sparkline_chunks[1]);
    } else {
        let fallback =
            Paragraph::new("No historical data available.\nReal-time values shown above.");
        frame.render_widget(fallback, sparkline_inner);
    }

    render_status_bar(frame, theme);
}
