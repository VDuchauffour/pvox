use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::{App, Modal};
use crate::theme::Theme;

/// Proxmox logo pixel art (2-row half-block encoding).
/// 'o' = primary color, 'd' = secondary color, '.' = empty.
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

/// Left offset matching the table's border + padding (1 + 1).
const TABLE_LEFT_OFFSET: u16 = 1;
const TABLE_RIGHT_OFFSET: u16 = 1;

/// Render the header section (info, keybindings, and logo).
pub fn render_header(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let shifted = Rect {
        x: area.x + TABLE_LEFT_OFFSET,
        width: area
            .width
            .saturating_sub(TABLE_LEFT_OFFSET + TABLE_RIGHT_OFFSET),
        ..area
    };

    let [info_area, keys_area, logo_area] = Layout::horizontal([
        Constraint::Length(64),
        Constraint::Length(40),
        Constraint::Min(5),
    ])
    .areas(shifted);

    render_header_info(frame, app, info_area, theme);
    if !matches!(app.modal, Some(Modal::Help)) {
        render_header_keys(frame, keys_area, theme);
    }
    render_header_logo(frame, app, logo_area, theme);
}

fn render_header_info(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let label_style = Style::default().fg(theme.warning);

    let host = app.config.host.as_deref().unwrap_or("n/a");
    let raw_user = if app.proxmox_user.is_empty() {
        app.config.token_id.as_deref().unwrap_or("n/a")
    } else {
        app.proxmox_user.as_str()
    };
    let user = raw_user.split('@').next().unwrap_or(raw_user);

    let value_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    // Compute cluster-wide CPU and RAM from node resources
    let nodes: Vec<_> = app
        .resources
        .iter()
        .filter(|r| r.r#type == "node")
        .collect();
    let (cpu_pct, ram_pct) = if !nodes.is_empty() {
        let total_cpu_weight: f64 = nodes.iter().filter_map(|n| n.maxcpu).sum();
        let weighted_cpu: f64 = nodes
            .iter()
            .filter_map(|n| n.cpu.zip(n.maxcpu).map(|(c, m)| c * m))
            .sum();
        let cpu_pct = if total_cpu_weight > 0.0 {
            format!("{:}%", (weighted_cpu / total_cpu_weight * 100.0).round())
        } else {
            "n/a".to_string()
        };
        let total_mem: u64 = nodes.iter().filter_map(|n| n.mem).sum();
        let total_maxmem: u64 = nodes.iter().filter_map(|n| n.maxmem).sum();
        let ram_pct = if total_maxmem > 0 {
            format!(
                "{:}%",
                (total_mem as f64 / total_maxmem as f64 * 100.0).round()
            )
        } else {
            "n/a".to_string()
        };
        (cpu_pct, ram_pct)
    } else {
        ("n/a".to_string(), "n/a".to_string())
    };

    let fields: &[(&str, String)] = &[
        ("Endpoint:", host.to_string()),
        ("Cluster:", "Proxmox VE".to_string()),
        ("User:", user.to_string()),
        ("P9S Rev:", env!("CARGO_PKG_VERSION").to_string()),
        (
            "PVE Rev:",
            if app.proxmox_version.is_empty() {
                "n/a".to_string()
            } else {
                app.proxmox_version.clone()
            },
        ),
        ("CPU:", cpu_pct),
        ("RAM:", ram_pct),
    ];

    let label_pad = fields.iter().map(|(l, _)| l.len()).max().unwrap_or(0) + 1;

    let lines: Vec<Line> = fields
        .iter()
        .map(|(label, value)| {
            Line::from(vec![
                Span::styled(format!("{label:<label_pad$}"), label_style),
                Span::styled(value.clone(), value_style),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_header_keys(frame: &mut Frame, area: Rect, theme: &Theme) {
    let key_style = theme.key_style();
    let label_style = Style::default().fg(theme.dim);

    let keys: &[(&str, &str)] = &[
        ("<?>", "Help"),
        ("</>", "Filter"),
        ("<:>", "Command"),
        ("<enter>", "Details"),
        ("<↑↓/jk>", "Scroll"),
        ("<gg/G>", "Top/Bottom"),
        ("<s>", "Start"),
        ("<S>", "Stop"),
        ("<r>", "Reboot"),
        ("<q>", "Quit"),
    ];

    const ROWS: usize = 6;

    let _global_key_pad = keys.iter().map(|(k, _)| k.len()).max().unwrap_or(0) + 1;

    let mut col_key_pad: Vec<usize> = Vec::new();
    let mut col_width: Vec<usize> = Vec::new();
    let mut total_width: usize = 0;
    let mut num_cols: usize = 0;

    for col in 0.. {
        let start = col * ROWS;
        if start >= keys.len() {
            break;
        }
        let end = ((col + 1) * ROWS).min(keys.len());
        let col_keys = &keys[start..end];
        let kp = col_keys.iter().map(|(k, _)| k.len()).max().unwrap_or(0) + 1;
        let cw = kp + col_keys.iter().map(|(_, v)| v.len()).max().unwrap_or(0) + 3;
        let needed = if num_cols == 0 {
            cw
        } else {
            total_width + 3 + cw
        };
        if needed > area.width as usize {
            break;
        }
        col_key_pad.push(kp);
        col_width.push(cw);
        total_width = needed;
        num_cols += 1;
    }

    if num_cols == 0 {
        num_cols = 1;
        let start = 0;
        let end = ROWS.min(keys.len());
        let col_keys = &keys[start..end];
        col_key_pad.push(col_keys.iter().map(|(k, _)| k.len()).max().unwrap_or(0) + 1);
        col_width
            .push(col_key_pad[0] + col_keys.iter().map(|(_, v)| v.len()).max().unwrap_or(0) + 3);
    }

    let cols = num_cols;

    let mut lines: Vec<Line> = Vec::with_capacity(ROWS);

    for row in 0..ROWS {
        let mut spans: Vec<Span> = Vec::new();
        for (col, (kp, cw)) in col_key_pad.iter().zip(col_width.iter()).enumerate() {
            let idx = col * ROWS + row;
            if idx < keys.len() {
                let kp = *kp;
                let cw = *cw;
                spans.push(Span::styled(format!("{:<kp$}", keys[idx].0), key_style));
                spans.push(Span::styled(keys[idx].1.to_string(), label_style));
                if col < cols - 1 {
                    let content_len = kp + keys[idx].1.len();
                    spans.push(Span::raw(" ".repeat(cw.saturating_sub(content_len))));
                }
            }
        }
        if !spans.is_empty() {
            lines.push(Line::from(spans));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_header_logo(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let no_color = app.config.no_color();

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

    frame.render_widget(Paragraph::new(logo_lines).alignment(Alignment::Right), area);
}
