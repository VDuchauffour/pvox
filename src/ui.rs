use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Sparkline, Table, TableState, Wrap},
};

use crate::app::{App, Modal};
use crate::client::ClusterResource;
use crate::event::ConfirmAction;
use crate::theme::Theme;

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

fn render_help(frame: &mut Frame, theme: &Theme) {
    let popup_area = centered_rect(60, 25, frame.area());
    frame.render_widget(Clear, popup_area);
    frame.render_widget(
        Paragraph::new(
            "Help\n\nq: quit\n?: help\n/: filter\n:: command (switch view)\n↑↓: scroll\nEnter: details\ns: start\nS: stop\nr: reboot",
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent))
                .title(Span::styled(" Help ", theme.accent_bold()))
                .title_alignment(Alignment::Center),
        ),
        popup_area.inner(Margin::new(1, 1)),
    );
}

fn command_error_rect(w: u16, h: u16, r: Rect) -> Rect {
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

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

fn render_filter(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let line = Line::from(vec![
        Span::styled(" / ", theme.prompt()),
        Span::raw(app.filter.clone()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent));

    frame.render_widget(Paragraph::new(line).block(block), area);
}

fn render_command(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
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

fn render_confirm(frame: &mut Frame, action: &ConfirmAction, theme: &Theme) {
    let area = frame.area();
    let msg = match action {
        ConfirmAction::Stop { node, vmid, .. } => format!("Stop {} on {}? (y/n)", vmid, node),
        ConfirmAction::Reboot { node, vmid, .. } => format!("Reboot {} on {}? (y/n)", vmid, node),
    };
    frame.render_widget(
        Paragraph::new(msg).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent))
                .title(Span::styled(" Confirm ", theme.accent_bold()))
                .title_alignment(Alignment::Center),
        ),
        area,
    );
}

fn render_command_error(frame: &mut Frame, command: &str, theme: &Theme) {
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

fn render_details(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = centered_rect(60, 70, frame.area());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(" Resource Details ", theme.accent_bold()))
        .title_alignment(Alignment::Center);

    let inner = area.inner(Margin::new(1, 1));
    let chunks = Layout::vertical([Constraint::Min(8), Constraint::Length(6)]).split(inner);

    let content = if let Some(resource) = app.current_resource() {
        match resource.r#type.as_str() {
            "qemu" | "lxc" => format_vm_details(resource),
            "node" => format_node_details(resource),
            "storage" => format_storage_details(resource),
            _ => format_generic_details(resource),
        }
    } else {
        "No resource selected".to_string()
    };

    let paragraph = Paragraph::new(content).wrap(Wrap { trim: true });
    frame.render_widget(Clear, area);
    frame.render_widget(block.clone(), area);
    frame.render_widget(paragraph, chunks[0]);

    let sparkline_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(" History ", theme.accent_bold()))
        .title_alignment(Alignment::Center);
    let sparkline_inner = chunks[1].inner(Margin::new(1, 1));
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
    frame.render_widget(sparkline_block, chunks[1]);
}

fn format_vm_details(r: &ClusterResource) -> String {
    let mut s = format!(
        "Name: {}\nType: {}\nNode: {}\nStatus: {}\n\n",
        r.name,
        r.r#type,
        r.node.as_ref().unwrap_or(&"N/A".to_string()),
        r.status
    );
    if let Some(cpu) = r.cpu {
        s.push_str(&format!("CPU: {:.1}%\n", cpu * 100.0));
    }
    if let (Some(mem), Some(maxmem)) = (r.mem, r.maxmem) {
        s.push_str(&format!(
            "Memory: {:.1} / {:.1} GB\n",
            mem as f64 / 1e9,
            maxmem as f64 / 1e9
        ));
    }
    s
}

fn format_node_details(r: &ClusterResource) -> String {
    format!(
        "Node: {}\nStatus: {}\nCPU: {:.1}%\nMemory: {:.1} / {:.1} GB\nUptime: {}s",
        r.name,
        r.status,
        r.cpu.unwrap_or(0.0) * 100.0,
        r.mem.unwrap_or(0) as f64 / 1e9,
        r.maxmem.unwrap_or(0) as f64 / 1e9,
        r.uptime.unwrap_or(0)
    )
}

fn format_storage_details(r: &ClusterResource) -> String {
    format!(
        "Storage: {}\nType: {}\nStatus: {}\nDisk: {} / {} GB",
        r.name,
        r.r#type,
        r.status,
        r.disk.unwrap_or(0) / (1024 * 1024 * 1024),
        r.maxdisk.unwrap_or(0) / (1024 * 1024 * 1024)
    )
}

fn format_generic_details(r: &ClusterResource) -> String {
    format!("Name: {}\nType: {}\nStatus: {}", r.name, r.r#type, r.status)
}

fn view_label(view: &str) -> &str {
    match view {
        "node" => "Nodes",
        "qemu" => "VMs",
        "lxc" => "Containers",
        "storage" => "Storage",
        other => other,
    }
}

const HEADER_HEIGHT: u16 = 7;

fn render_header(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let no_color = app.config.no_color;

    let [info_area, keys_area, logo_area] = Layout::horizontal([
        Constraint::Length(34),
        Constraint::Length(22),
        Constraint::Min(0),
    ])
    .areas(area);

    render_header_info(frame, app, info_area, theme);
    render_header_keys(frame, keys_area, theme);

    let pixel = |c: char| -> Option<Color> {
        match c {
            'o' => Some(theme.logo_primary),
            'd' => Some(theme.logo_secondary),
            _ => None,
        }
    };

    let logo_lines: Vec<Line> = PROXMOX_PIXELS
        .chunks(2)
        .map(|pair| {
            let top: Vec<char> = pair[0].chars().collect();
            let bottom: Vec<char> = pair[1].chars().collect();
            let spans = (0..top.len())
                .map(|x| {
                    let t = pixel(top[x]);
                    let b = pixel(*bottom.get(x).unwrap_or(&'.'));
                    match (t, b) {
                        (None, None) => Span::raw(" "),
                        (Some(tc), None) => Span::styled("▀", Style::default().fg(tc)),
                        (None, Some(bc)) => Span::styled("▄", Style::default().fg(bc)),
                        (Some(_), Some(_)) if no_color => Span::raw("█"),
                        (Some(tc), Some(bc)) => Span::styled("▀", Style::default().fg(tc).bg(bc)),
                    }
                })
                .collect::<Vec<_>>();
            Line::from(spans)
        })
        .collect();

    frame.render_widget(
        Paragraph::new(logo_lines).alignment(Alignment::Right),
        logo_area,
    );
}

fn render_header_info(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let label_style = theme.label();

    let host = app.config.host.as_deref().unwrap_or("n/a");
    let user = app.config.token_id.as_deref().unwrap_or("n/a");
    let filter = if app.filter.is_empty() {
        "<none>".to_string()
    } else {
        app.filter.clone()
    };

    let (conn_text, conn_color) = if app.connected {
        ("Connected", theme.success)
    } else {
        ("Disconnected", theme.danger)
    };
    let conn_span = Span::styled(conn_text, Style::default().fg(conn_color));

    let field = |label: &str, value: String| {
        Line::from(vec![
            Span::styled(format!("{label:<11}"), label_style),
            Span::raw(value),
        ])
    };

    let res_label = format!("{}:", view_label(&app.view));

    let lines = vec![
        Line::from(vec![
            Span::styled(format!("{:<11}", "Status:"), label_style),
            conn_span,
        ]),
        field("Host:", host.to_string()),
        field("User:", user.to_string()),
        field("Cluster:", "Proxmox VE".to_string()),
        field("Refresh:", format!("{}s", app.config.refresh_interval)),
        field(&res_label, app.display_resources.len().to_string()),
        field("Filter:", filter),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_header_keys(frame: &mut Frame, area: Rect, theme: &Theme) {
    let key_style = theme.key_style();

    let binding = |key: &'static str, label: &'static str| {
        Line::from(vec![
            Span::styled(format!("{key:<8}"), key_style),
            Span::raw(label),
        ])
    };

    let lines = vec![
        binding("<?>", "Help"),
        binding("</>", "Filter"),
        binding("<:>", "Command"),
        binding("<enter>", "Details"),
        binding("<s>", "Start"),
        binding("<S>", "Stop"),
        binding("<r>", "Reboot"),
        binding("<q>", "Quit"),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_list(frame: &mut Frame, app: &App, theme: &Theme) {
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
        Constraint::Min(8),  // Type
        Constraint::Min(15), // Name
        Constraint::Min(10), // Node
        Constraint::Min(10), // Status
        Constraint::Min(8),  // CPU%
        Constraint::Min(12), // RAM
        Constraint::Min(12), // Disk
    ];

    let header = Row::new(vec![
        "Type", "Name", "Node", "Status", "CPU%", "RAM", "Disk",
    ])
    .style(theme.accent_bold());

    let rows: Vec<Row> = app
        .display_resources
        .iter()
        .map(|r| {
            let status_style = if no_color {
                Style::default()
            } else {
                match r.status.as_str() {
                    "running" | "online" => theme.status_running(),
                    "stopped" => theme.status_stopped(),
                    _ => theme.status_warning(),
                }
            };

            let cpu_str = r
                .cpu
                .map(|c| format!("{:.1}%", c * 100.0))
                .unwrap_or_else(|| "-".to_string());
            let ram_str = format_memory(r.mem, r.maxmem);
            let disk_str = format_disk(r.disk, r.maxdisk);

            Row::new(vec![
                Cell::from(r.r#type.clone()),
                Cell::from(r.name.clone()),
                Cell::from(r.node.clone().unwrap_or_default()),
                Cell::from(r.status.clone()).style(status_style),
                Cell::from(cpu_str),
                Cell::from(ram_str),
                Cell::from(disk_str),
            ])
        })
        .collect();

    let mut table_state = TableState::default();
    table_state.select(Some(app.selected_index));

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent))
                .title(Span::styled(
                    format!(" {} ", view_label(&app.view)),
                    theme.accent_bold(),
                ))
                .title_alignment(Alignment::Center),
        )
        .row_highlight_style(if no_color {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            theme.row_highlight()
        });

    frame.render_stateful_widget(table, table_area, &mut table_state);

    render_status_bar(frame, app, theme);
}

fn render_status_bar(frame: &mut Frame, app: &App, theme: &Theme) {
    let no_color = app.config.no_color;

    let area = frame.area();
    let status_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };

    let (conn_text, conn_color) = if app.connected {
        ("Connected", theme.success)
    } else {
        ("Disconnected", theme.danger)
    };

    let selected_text = if app.display_resources.is_empty() {
        "0/0".to_string()
    } else {
        format!(
            "{}/{} ({})",
            app.selected_index + 1,
            app.display_resources.len(),
            app.selected_resource()
                .map(|r| r.r#type.clone())
                .unwrap_or_default(),
        )
    };

    let status_msg = app.status_message.as_deref().unwrap_or("");

    let status_span = if no_color {
        Span::raw(conn_text)
    } else {
        Span::styled(conn_text, Style::default().fg(conn_color))
    };

    let mut spans: Vec<Span> = vec![];
    spans.push(Span::raw("["));
    spans.push(status_span);
    spans.push(Span::raw("] "));

    if !status_msg.is_empty() {
        spans.push(Span::raw(status_msg));
    }

    let view_label = match &app.modal {
        Some(Modal::Help) => "Help",
        Some(Modal::Filter) => "Filter",
        Some(Modal::Command) => "Command",
        Some(Modal::CommandError(_)) => "Error",
        Some(Modal::Confirm(_)) => "Confirm",
        Some(Modal::Details) => "Details",
        None => view_label(&app.view),
    };

    spans.push(Span::raw(" | "));
    spans.push(Span::styled(
        format!("[{}]", view_label),
        if no_color {
            Style::default()
        } else {
            Style::default().fg(theme.accent)
        },
    ));
    spans.push(Span::raw(" "));
    spans.push(Span::raw(selected_text));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(Text::from(line));
    frame.render_widget(paragraph, status_area);
}

const PROXMOX_PIXELS: [&str; 14] = [
    "...ddd....ddd...",
    "...dddd..dddd...",
    "ooo.dddddddd.ooo",
    ".ooo.dddddd.ooo.",
    "..ooo.dddd.ooo..",
    "...ooo.dd.ooo...",
    "....ooo..ooo....",
    "....ooo..ooo....",
    "...ooo.dd.ooo...",
    "..ooo.dddd.ooo..",
    ".ooo.dddddd.ooo.",
    "ooo.dddddddd.ooo",
    "...dddd..dddd...",
    "...ddd....ddd...",
];

fn format_memory(used: Option<u64>, total: Option<u64>) -> String {
    match (used, total) {
        (Some(u), Some(t)) => format!("{:.1} / {:.1} GB", u as f64 / 1e9, t as f64 / 1e9),
        _ => "-".to_string(),
    }
}

fn format_disk(used: Option<u64>, total: Option<u64>) -> String {
    match (used, total) {
        (Some(u), Some(t)) => {
            format!(
                "{} / {} GB",
                u / (1024 * 1024 * 1024),
                t / (1024 * 1024 * 1024)
            )
        }
        _ => "-".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_label_maps_known_types() {
        assert_eq!(view_label("node"), "Nodes");
        assert_eq!(view_label("qemu"), "VMs");
        assert_eq!(view_label("lxc"), "Containers");
        assert_eq!(view_label("storage"), "Storage");
    }

    #[test]
    fn view_label_passes_through_unknown() {
        assert_eq!(view_label("sdn"), "sdn");
    }

    #[test]
    fn theme_default_has_blue_accent() {
        let t = Theme::default_theme();
        assert_eq!(t.accent, Color::Blue);
    }

    #[test]
    fn theme_no_color_uses_reset() {
        let t = Theme::no_color();
        assert_eq!(t.accent, Color::Reset);
        assert_eq!(t.success, Color::Reset);
        assert_eq!(t.danger, Color::Reset);
    }

    #[test]
    fn theme_from_no_color_flag() {
        let colored = Theme::from_no_color(false);
        assert_eq!(colored.accent, Color::Blue);
        let plain = Theme::from_no_color(true);
        assert_eq!(plain.accent, Color::Reset);
    }
}
