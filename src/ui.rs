use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Sparkline, Table, TableState, Wrap},
    Frame,
};

use crate::app::{App, Modal};
use crate::client::ClusterResource;
use crate::event::ConfirmAction;

pub fn render(frame: &mut Frame, app: &App) {
    match &app.modal {
        Some(Modal::Help) => render_help(frame, app),
        Some(Modal::Filter) => {
            render_list(frame, app); // Show list behind filter
            render_filter(frame, app);
        }
        Some(Modal::Confirm(action)) => {
            render_list(frame, app);
            render_confirm(frame, action, app);
        }
        Some(Modal::Details) => render_details(frame, app),
        None => render_list(frame, app),
    }
}

fn render_help(frame: &mut Frame, _app: &App) {
    let popup_area = centered_rect(60, 25, frame.area());
    frame.render_widget(Clear, popup_area);
    frame.render_widget(
        Paragraph::new(
            "Help\n\nq: quit\n?: help\n/: filter\n↑↓: scroll\nEnter: details\ns: start\nS: stop\nr: reboot",
        )
        .block(Block::default().borders(Borders::ALL).title("Help")),
        popup_area.inner(Margin::new(1, 1)),
    );
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

fn render_filter(frame: &mut Frame, app: &App) {
    render_list(frame, app);

    let area = frame.area();
    let input_area = Rect {
        x: area.x,
        y: area.height.saturating_sub(3),
        width: area.width,
        height: 3,
    };

    let block = Block::default().borders(Borders::ALL).title("Filter");

    let text = format!("Filter: {}", app.filter);
    let paragraph = Paragraph::new(text).block(block);

    frame.render_widget(paragraph, input_area);
}

fn render_confirm(frame: &mut Frame, action: &ConfirmAction, _app: &App) {
    let area = frame.area();
    let msg = match action {
        ConfirmAction::Stop { node, vmid, .. } => format!("Stop {} on {}? (y/n)", vmid, node),
        ConfirmAction::Reboot { node, vmid, .. } => format!("Reboot {} on {}? (y/n)", vmid, node),
    };
    frame.render_widget(
        Paragraph::new(msg).block(Block::default().borders(Borders::ALL).title("Confirm")),
        area,
    );
}

fn render_details(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 70, frame.area());
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Resource Details");

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

    let sparkline_block = Block::default().borders(Borders::ALL).title("History");
    let sparkline_inner = chunks[1].inner(Margin::new(1, 1));
    let sparkline_chunks = Layout::vertical([Constraint::Length(1), Constraint::Length(1)])
        .margin(1)
        .split(sparkline_inner);

    if !app.sparkline_data.cpu_history.is_empty() {
        let cpu_sparkline = Sparkline::default()
            .data(&app.sparkline_data.cpu_history)
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(cpu_sparkline, sparkline_chunks[0]);
        let mem_sparkline = Sparkline::default()
            .data(&app.sparkline_data.mem_history)
            .style(Style::default().fg(Color::Cyan));
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

fn render_list(frame: &mut Frame, app: &App) {
    let no_color = app.config.no_color;
    let area = frame.area();
    let main_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height.saturating_sub(1),
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
    .style(if no_color {
        Style::default()
    } else {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    });

    let rows: Vec<Row> = app
        .display_resources
        .iter()
        .map(|r| {
            let status_style = if no_color {
                Style::default()
            } else {
                match r.status.as_str() {
                    "running" | "online" => Style::default().fg(Color::Green),
                    "stopped" => Style::default().fg(Color::Red),
                    _ => Style::default().fg(Color::Yellow),
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
        .block(Block::default().borders(Borders::ALL).title("Resources"))
        .row_highlight_style(if no_color {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().bg(Color::Blue).fg(Color::White)
        });

    frame.render_stateful_widget(table, main_area, &mut table_state);

    render_status_bar(frame, app);
}

fn render_status_bar(frame: &mut Frame, app: &App) {
    let no_color = app.config.no_color;

    let area = frame.area();
    let status_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };

    let (conn_text, conn_color) = if app.connected {
        ("Connected", Color::Green)
    } else {
        ("Disconnected", Color::Red)
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
        Some(Modal::Confirm(_)) => "Confirm",
        Some(Modal::Details) => "Details",
        None => "Resources",
    };

    spans.push(Span::raw(" | "));
    spans.push(Span::styled(
        format!("[{}]", view_label),
        if no_color {
            Style::default()
        } else {
            Style::default().fg(Color::Yellow)
        },
    ));
    spans.push(Span::raw(" "));
    spans.push(Span::raw(selected_text));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(Text::from(line));
    frame.render_widget(paragraph, status_area);
}

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
