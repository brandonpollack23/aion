use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::config::schema::ViewMode;
use crate::state::app_state::AppState;

pub fn render_header(app: &AppState, frame: &mut Frame, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");
    let title = format!("aion v{}", version);
    let hints = "  /:search  C:calendars  ?:help";

    let login_indicator = if app.is_auth_loading {
        Span::styled(" ⟳ ", Style::default().fg(app.theme.accent_warning))
    } else if app.is_logged_in {
        Span::styled(" ● ", Style::default().fg(app.theme.accent_success))
    } else {
        Span::styled(" ○ ", Style::default().fg(app.theme.text_dim))
    };

    let sync_indicator = if app.is_syncing {
        Span::styled("⟳ ", Style::default().fg(app.theme.accent_warning))
    } else {
        Span::raw("  ")
    };

    let mut spans = vec![
        Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
        login_indicator,
        sync_indicator,
        Span::styled(hints, Style::default().fg(app.theme.text_dim)),
    ];

    if app.show_timezone {
        spans.push(Span::styled(
            format!("  [{}]", app.timezone),
            Style::default().fg(app.theme.accent_primary),
        ));
    }

    let view_label = match app.view_mode {
        ViewMode::Monthly => Some("Month"),
        ViewMode::Daily => match app.column_count {
            3 => Some("3-day"),
            5 => Some("5-day"),
            _ => None,
        },
    };
    if let Some(label) = view_label {
        spans.push(Span::styled(
            format!("  {}", label),
            Style::default().fg(app.theme.text_dim),
        ));
    }

    let line = Line::from(spans);

    frame.render_widget(Paragraph::new(line), area);
}
