use ratatui::widgets::Paragraph;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

use super::format::{format_disk, format_memory, view_label};
use super::header::render_header;
use super::overlays::{render_command, render_filter};
use crate::app::{App, Modal};
use crate::theme::Theme;

/// Height reserved for the header section (info + keybindings + logo).
pub(super) const HEADER_HEIGHT: u16 = 7;

/// Render the main resource list view, including header, optional filter/command bar, and table.
pub fn render_list(frame: &mut Frame, app: &App, theme: &Theme) {
    let no_color = app.config.no_color;
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

    let table_area = if matches!(app.modal, Some(Modal::Filter)) {
        let [filter_area, rest] =
            Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(main_area);
        render_filter(frame, app, filter_area, theme);
        rest
    } else if matches!(app.modal, Some(Modal::Command)) {
        let [command_area, rest] =
            Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(main_area);
        render_command(frame, app, command_area, theme);
        rest
    } else {
        main_area
    };

    let widths = [
        Constraint::Min(15), // Name
        Constraint::Min(10), // Node
        Constraint::Min(10), // Status
        Constraint::Min(8),  // CPU%
        Constraint::Min(12), // RAM
        Constraint::Min(12), // Disk
    ];

    let header = Row::new(vec!["NAME", "NODE", "STATUS", "CPU%", "RAM", "DISK"]).style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .display_resources
        .iter()
        .map(|r| {
            let accent_style = if no_color {
                Style::default()
            } else {
                Style::default().fg(theme.accent)
            };

            let cpu_str = r
                .cpu
                .map(|c| format!("{:.1}%", c * 100.0))
                .unwrap_or_else(|| "-".to_string());
            let ram_str = format_memory(r.mem, r.maxmem);
            let disk_str = format_disk(r.disk, r.maxdisk);

            Row::new(vec![
                Cell::from(r.name.clone()).style(accent_style),
                Cell::from(r.node.clone().unwrap_or_default()).style(accent_style),
                Cell::from(r.status.clone()).style(accent_style),
                Cell::from(cpu_str).style(accent_style),
                Cell::from(ram_str).style(accent_style),
                Cell::from(disk_str).style(accent_style),
            ])
        })
        .collect();

    let mut table_state = TableState::default();
    table_state.select(Some(app.selected_index));

    let title = if app.filter.is_empty() {
        Line::from(Span::styled(
            format!(" {} ", view_label(&app.view)),
            theme.accent_bold(),
        ))
    } else {
        Line::from(vec![
            Span::styled(format!(" {} ", view_label(&app.view)), theme.accent_bold()),
            Span::styled(
                format!("</{}> ", app.filter),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent))
                .title(title)
                .title_alignment(ratatui::layout::Alignment::Center)
                .padding(ratatui::widgets::Padding::new(1, 1, 0, 0)),
        )
        .row_highlight_style(if no_color {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::Black).bg(theme.accent)
        });

    let table_area = table_area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 0,
    });

    frame.render_stateful_widget(table, table_area, &mut table_state);

    render_status_bar(frame, theme);
}

pub(super) fn render_status_bar(frame: &mut Frame, _theme: &Theme) {
    let area = frame.area();
    let status_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };

    frame.render_widget(Paragraph::new(Line::from("")), status_area);
}
