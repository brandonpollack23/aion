use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::state::app_state::AppState;
use crate::state::message::MessageType;
use crate::ui::layout_helpers::centered_fixed;

pub fn render_messages_dialog(app: &AppState, frame: &mut Frame, area: Rect) {
    let width = 72u16.min(area.width);
    let height = (area.height * 4 / 5).max(8);
    let rect = centered_fixed(width, height, area);

    frame.render_widget(Clear, rect);

    let count = app.message_log.len();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent_primary))
        .title(Span::styled(
            " Messages ",
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            " c:clear  Esc:close ",
            Style::default().fg(app.theme.text_dim),
        ));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    if inner.height == 0 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    if app.message_log.is_empty() {
        lines.push(Line::from(Span::styled(
            "No messages yet.",
            Style::default().fg(app.theme.text_dim),
        )));
    } else {
        // Newest first
        for msg in app.message_log.iter().rev() {
            let (icon, color) = match msg.msg_type {
                MessageType::Success => ("✓", app.theme.accent_success),
                MessageType::Warning => ("⚠", app.theme.accent_warning),
                MessageType::Error => ("✗", app.theme.accent_error),
                MessageType::Progress => ("⋯", app.theme.accent_primary),
                MessageType::Info => ("·", app.theme.text_secondary),
            };

            let elapsed = msg.created_at.elapsed();
            let time_str = if elapsed.as_secs() < 60 {
                format!("{:>3}s ago", elapsed.as_secs())
            } else if elapsed.as_secs() < 3600 {
                format!("{:>3}m ago", elapsed.as_secs() / 60)
            } else {
                format!("{:>3}h ago", elapsed.as_secs() / 3600)
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<8} ", time_str),
                    Style::default().fg(app.theme.text_dim),
                ),
                Span::styled(format!("{} ", icon), Style::default().fg(color)),
                Span::styled(
                    msg.text.clone(),
                    Style::default().fg(match msg.msg_type {
                        MessageType::Error => color,
                        _ => app.theme.text_primary,
                    }),
                ),
            ]));
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!(
                "{} message{} (newest first)",
                count,
                if count == 1 { "" } else { "s" }
            ),
            Style::default().fg(app.theme.text_dim),
        )));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
