use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::header::render_header;
use super::table::{HEADER_HEIGHT, render_status_bar};
use crate::app::App;
use crate::theme::Theme;

type Binding = (&'static str, &'static str);

struct HelpColumn {
    header: &'static str,
    bindings: &'static [Binding],
}

pub fn render_help(frame: &mut Frame, app: &App, theme: &Theme) {
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
        .title(Span::styled(" Help ", theme.accent_bold()))
        .title_alignment(Alignment::Center)
        .padding(ratatui::widgets::Padding::new(1, 1, 0, 0));

    let inner = block.inner(main_area);
    frame.render_widget(block, main_area);

    let resource: &[Binding] = &[
        ("<:qemu>", "VMs"),
        ("<:lxc>", "Containers"),
        ("<:node>", "Nodes"),
        ("<:storage>", "Storage"),
        ("<:pool>", "Pools"),
        ("<:sdn>", "SDN"),
        ("<:task>", "Tasks"),
        ("<:replication>", "Replication"),
        ("<:ha>", "HA"),
        ("<:backup>", "Backups"),
        ("<:disk>", "Disks"),
    ];

    let general: &[Binding] = &[
        ("<?>", "Help"),
        ("</>", "Filter"),
        ("<:>", "Command"),
        ("<q>", "Quit"),
        ("<esc>", "Back"),
        ("<ctrl-c>", "Force Quit"),
        ("<s>", "Start"),
        ("<S>", "Stop"),
        ("<r>", "Reboot"),
        ("<Enter>", "View"),
    ];

    let navigation: &[Binding] = &[
        ("<↑/k>", "Up"),
        ("<↓/j>", "Down"),
        ("<gg>", "Top"),
        ("<G>", "Bottom"),
    ];

    let columns = [
        HelpColumn {
            header: "RESOURCE",
            bindings: resource,
        },
        HelpColumn {
            header: "GENERAL",
            bindings: general,
        },
        HelpColumn {
            header: "NAVIGATION",
            bindings: navigation,
        },
    ];

    let col_areas = Layout::horizontal([
        Constraint::Percentage(38),
        Constraint::Percentage(32),
        Constraint::Percentage(30),
    ])
    .split(inner);

    let key_style = theme.key_style();
    let desc_style = Style::default().fg(Color::White);
    let header_style = theme.accent_bold();

    for (i, col) in columns.iter().enumerate() {
        let col_area = col_areas[i];
        let key_pad = col.bindings.iter().map(|(k, _)| k.len()).max().unwrap_or(0) + 1;

        let mut lines: Vec<Line> = vec![
            Line::from(Span::styled(col.header, header_style)),
            Line::from(""),
        ];

        for (key, desc) in col.bindings {
            lines.push(Line::from(vec![
                Span::styled(format!("{:<width$}", key, width = key_pad), key_style),
                Span::styled((*desc).to_string(), desc_style),
            ]));
        }

        frame.render_widget(Paragraph::new(lines), col_area);
    }

    render_status_bar(frame, theme);
}
