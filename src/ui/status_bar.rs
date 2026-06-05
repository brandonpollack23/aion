use chrono::Local;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::config::schema::ViewMode;
use crate::state::app_state::AppState;
use crate::state::message::MessageType;

pub fn render_status_bar(app: &AppState, frame: &mut Frame, area: Rect) {
    // Show message if present, otherwise show clock + hints
    let line = if let Some(ref msg) = app.message {
        let color = match msg.msg_type {
            MessageType::Success => app.theme.accent_success,
            MessageType::Warning => app.theme.accent_warning,
            MessageType::Error => app.theme.accent_error,
            MessageType::Progress => app.theme.accent_primary,
            MessageType::Info => app.theme.text_primary,
        };
        let prefix = match msg.msg_type {
            MessageType::Success => "✓ ",
            MessageType::Warning => "⚠ ",
            MessageType::Error => "✗ ",
            MessageType::Progress => "⟳ ",
            MessageType::Info => "",
        };
        // Show progress if available
        let text = if let Some(ref p) = msg.progress {
            match (p.current, p.total) {
                (Some(c), Some(t)) => format!("{}{} [{}/{}]", prefix, msg.text, c, t),
                _ => format!("{}{}", prefix, msg.text),
            }
        } else {
            format!("{}{}", prefix, msg.text)
        };
        Line::from(vec![Span::styled(text, Style::default().fg(color))])
    } else {
        let now = Local::now();
        let time_str = format!("{}", now.format("%H:%M"));
        let date_str = format!("{}", now.format("%a %b %e"));
        let hints = match app.view_mode {
            ViewMode::Monthly => "  hjkl:move  H/L:month  n:today  Enter:day view  q:quit",
            ViewMode::Daily => "  n:now  j/k:events  h/l:days  q:quit",
        };

        let invite_count = app.pending_invites().len();
        let mut spans = vec![
            Span::styled(time_str, Style::default().fg(app.theme.accent_primary)),
            Span::styled("  ", Style::default()),
            Span::styled(date_str, Style::default().fg(app.theme.text_secondary)),
            Span::styled(hints, Style::default().fg(app.theme.text_dim)),
        ];
        if invite_count > 0 {
            spans.push(Span::styled(
                format!("  ! {} invite{}", invite_count, if invite_count == 1 { "" } else { "s" }),
                Style::default().fg(app.theme.accent_warning),
            ));
        }
        Line::from(spans)
    };

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(app.theme.statusbar_bg)),
        area,
    );
}
