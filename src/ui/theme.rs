use ratatui::style::Color;

use crate::config::schema::Theme as ConfigTheme;

pub struct AppTheme {
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_dim: Color,
    pub text_weekend: Color,
    pub accent_primary: Color,
    pub accent_success: Color,
    pub accent_warning: Color,
    pub accent_error: Color,
    pub event_default: Color,
    pub event_out_of_office: Color,
    pub event_focus_time: Color,
    pub event_birthday: Color,
    pub selection_bg: Color,
    pub selection_text: Color,
    pub status_accepted: Color,
    pub status_declined: Color,
    pub status_tentative: Color,
    pub status_needs_action: Color,
    pub modal_bg: Color,
    pub modal_border: Color,
    pub input_bg: Color,
    pub input_text: Color,
    pub input_placeholder: Color,
    pub statusbar_bg: Color,
    pub statusbar_text: Color,
    pub timezone_local_bg: Color,
    // Calendar color palette (for calendar key hashing)
    pub calendar_palette: Vec<Color>,
}

impl AppTheme {
    pub fn from_config(cfg: &ConfigTheme) -> Self {
        // Build palette from config calendar_colors map (keys "1"–"N" in order)
        let mut palette_keys: Vec<usize> = cfg
            .calendar_colors
            .keys()
            .filter_map(|k| k.parse::<usize>().ok())
            .collect();
        palette_keys.sort();
        let calendar_palette: Vec<Color> = if palette_keys.is_empty() {
            vec![
                Color::Blue,
                Color::Green,
                Color::Magenta,
                Color::Red,
                Color::Yellow,
                Color::Cyan,
            ]
        } else {
            palette_keys
                .iter()
                .filter_map(|k| cfg.calendar_colors.get(&k.to_string()))
                .map(|s| parse_color(s))
                .collect()
        };

        Self {
            text_primary: parse_color(&cfg.text.primary),
            text_secondary: parse_color(&cfg.text.secondary),
            text_dim: parse_color(&cfg.text.dim),
            text_weekend: parse_color(&cfg.text.weekend),
            accent_primary: parse_color(&cfg.accent.primary),
            accent_success: parse_color(&cfg.accent.success),
            accent_warning: parse_color(&cfg.accent.warning),
            accent_error: parse_color(&cfg.accent.error),
            event_default: parse_color(&cfg.event_type.default),
            event_out_of_office: parse_color(&cfg.event_type.out_of_office),
            event_focus_time: parse_color(&cfg.event_type.focus_time),
            event_birthday: parse_color(&cfg.event_type.birthday),
            selection_bg: parse_color(&cfg.selection.background),
            selection_text: parse_color(&cfg.selection.text),
            status_accepted: parse_color(&cfg.status.accepted),
            status_declined: parse_color(&cfg.status.declined),
            status_tentative: parse_color(&cfg.status.tentative),
            status_needs_action: parse_color(&cfg.status.needs_action),
            modal_bg: parse_color(&cfg.modal.background),
            modal_border: parse_color(&cfg.modal.border),
            input_bg: parse_color(&cfg.input.background),
            input_text: parse_color(&cfg.input.text),
            input_placeholder: parse_color(&cfg.input.placeholder),
            statusbar_bg: parse_color(&cfg.status_bar.background),
            statusbar_text: parse_color(&cfg.status_bar.text),
            timezone_local_bg: parse_color(&cfg.timezone_local_bg),
            calendar_palette,
        }
    }

    pub fn calendar_color(&self, calendar_key: &str) -> Color {
        if self.calendar_palette.is_empty() {
            return Color::Cyan;
        }
        let hash: usize = calendar_key
            .bytes()
            .fold(0usize, |acc, b| acc.wrapping_add(b as usize));
        self.calendar_palette[hash % self.calendar_palette.len()]
    }
}

fn parse_color(s: &str) -> Color {
    // Handle hex colors
    if s.starts_with('#') && s.len() == 7 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&s[1..3], 16),
            u8::from_str_radix(&s[3..5], 16),
            u8::from_str_radix(&s[5..7], 16),
        ) {
            return Color::Rgb(r, g, b);
        }
    }
    match s {
        "black" => Color::Black,
        "blackBright" | "brightBlack" | "darkGray" => Color::DarkGray,
        "red" => Color::Red,
        "redBright" | "brightRed" => Color::LightRed,
        "green" => Color::Green,
        "greenBright" | "brightGreen" => Color::LightGreen,
        "yellow" => Color::Yellow,
        "yellowBright" | "brightYellow" => Color::LightYellow,
        "blue" => Color::Blue,
        "blueBright" | "brightBlue" => Color::LightBlue,
        "magenta" => Color::Magenta,
        "magentaBright" | "brightMagenta" => Color::LightMagenta,
        "cyan" => Color::Cyan,
        "cyanBright" | "brightCyan" => Color::LightCyan,
        "white" => Color::Gray,
        "whiteBright" | "brightWhite" => Color::White,
        _ => Color::Reset,
    }
}
