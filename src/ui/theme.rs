use ratatui::style::{Color, Modifier, Style};
use std::sync::LazyLock;

pub static THEME: LazyLock<Theme> = LazyLock::new(Theme::default);

#[derive(Clone, Debug)]
pub struct Theme {
    // Borders
    pub border_focused: Color,
    pub border_unfocused: Color,

    // Text
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_accent: Color,
    pub text_emphasis: Color,
    pub text_success: Color,
    pub text_danger: Color,
    pub text_link: Color,

    // Status bar
    pub status_bar_bg: Color,
    pub status_bar_accent_fg: Color,
    pub status_bar_accent_bg: Color,

    // Input / Confirm overlays
    pub input_border: Color,
    pub input_text: Color,
    pub confirm_border: Color,
    pub confirm_text: Color,
    pub message_fg: Color,

    // Heatmap (5-level gradient)
    pub heat_colors: [Color; 5],

    // Model chart palette
    pub model_colors: [Color; 6],

    // Charts
    pub chart_bar: Color,
    pub chart_label: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border_focused: Color::Cyan,
            border_unfocused: Color::DarkGray,

            text_primary: Color::Reset,
            text_secondary: Color::DarkGray,
            text_accent: Color::Cyan,
            text_emphasis: Color::Yellow,
            text_success: Color::Green,
            text_danger: Color::Red,
            text_link: Color::Blue,

            status_bar_bg: Color::DarkGray,
            status_bar_accent_fg: Color::Black,
            status_bar_accent_bg: Color::Cyan,

            input_border: Color::Cyan,
            input_text: Color::Reset,
            confirm_border: Color::Yellow,
            confirm_text: Color::Yellow,
            message_fg: Color::Green,

            heat_colors: [
                Color::DarkGray,
                Color::Green,
                Color::Green,
                Color::LightGreen,
                Color::LightGreen,
            ],

            model_colors: [
                Color::Cyan,
                Color::Green,
                Color::Yellow,
                Color::Magenta,
                Color::Blue,
                Color::Red,
            ],

            chart_bar: Color::Cyan,
            chart_label: Color::DarkGray,
        }
    }
}

impl Theme {
    pub fn highlight_style(&self) -> Style {
        Style::default()
            .bg(self.border_unfocused)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }
}
