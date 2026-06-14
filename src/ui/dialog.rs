use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Sparkline, Wrap},
};

use super::layout::{centered_rect, command_error_rect};
use crate::api::ClusterResource;
use crate::app::App;
use crate::event::ConfirmAction;
use crate::theme::Theme;

/// Render the help overlay popup.
pub fn render_help(frame: &mut Frame, theme: &Theme) {
    let popup_area = centered_rect(60, 25, frame.area());
    frame.render_widget(Clear, popup_area);
    frame.render_widget(
        Paragraph::new(
            "Help\n\nq: quit\n?: help\n/: filter\n:: command (switch view)\n↑↓/jk: scroll\ngg: go to top\nG: go to bottom\nEnter: details\ns: start\nS: stop\nr: reboot",
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

/// Render a confirmation dialog for stop/reboot actions.
pub fn render_confirm(frame: &mut Frame, action: &ConfirmAction, theme: &Theme) {
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

/// Render the resource details modal overlay with sparkline history.
pub fn render_details(frame: &mut Frame, app: &App, theme: &Theme) {
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
