use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub accent: Color,
    pub success: Color,
    pub danger: Color,
    pub warning: Color,
    pub dim: Color,
    pub key_binding: Color,
    pub highlight_fg: Color,
    pub highlight_bg: Color,
    pub logo_primary: Color,
    pub logo_secondary: Color,
}

impl Theme {
    pub fn default_theme() -> Self {
        Self {
            accent: Color::Blue,
            success: Color::Green,
            danger: Color::Red,
            warning: Color::Yellow,
            dim: Color::DarkGray,
            key_binding: Color::LightBlue,
            highlight_fg: Color::White,
            highlight_bg: Color::Blue,
            logo_primary: Color::Rgb(229, 112, 0),
            logo_secondary: Color::Rgb(214, 214, 214),
        }
    }

    pub fn no_color() -> Self {
        Self {
            accent: Color::Reset,
            success: Color::Reset,
            danger: Color::Reset,
            warning: Color::Reset,
            dim: Color::Reset,
            key_binding: Color::Reset,
            highlight_fg: Color::Reset,
            highlight_bg: Color::Reset,
            logo_primary: Color::Reset,
            logo_secondary: Color::Reset,
        }
    }

    pub fn from_no_color(no_color: bool) -> Self {
        if no_color {
            Self::no_color()
        } else {
            Self::default_theme()
        }
    }

    pub fn label(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn prompt(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn bold(&self) -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }

    pub fn accent_bold(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn accent_bg_bold(&self) -> Style {
        Style::default()
            .bg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn completion(&self) -> Style {
        Style::default().fg(self.dim)
    }

    pub fn completion_no_color(&self) -> Style {
        Style::default().add_modifier(Modifier::DIM)
    }

    pub fn key_style(&self) -> Style {
        Style::default()
            .fg(self.key_binding)
            .add_modifier(Modifier::BOLD)
    }

    pub fn key_style_no_color(&self) -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }

    pub fn row_highlight(&self) -> Style {
        Style::default().bg(self.highlight_bg).fg(self.highlight_fg)
    }

    pub fn status_running(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn status_stopped(&self) -> Style {
        Style::default().fg(self.danger)
    }

    pub fn status_warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn sparkline_cpu(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn sparkline_mem(&self) -> Style {
        Style::default().fg(self.accent)
    }
}
