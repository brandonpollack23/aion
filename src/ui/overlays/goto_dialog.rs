use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::state::app_state::AppState;
use crate::ui::layout_helpers::centered_fixed;

pub fn render_goto_dialog(app: &AppState, frame: &mut Frame, area: Rect) {
    let width = 50u16.min(area.width);
    let height = 6u16;
    let rect = centered_fixed(width, height, area);

    frame.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent_primary))
        .title(Span::styled(
            " Go to Date ",
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            " Enter:go  Esc:cancel ",
            Style::default().fg(app.theme.text_dim),
        ));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    if inner.height == 0 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // Input line
    lines.push(Line::from(vec![
        Span::styled("> ", Style::default().fg(app.theme.accent_primary)),
        Span::styled(
            app.goto_input.clone(),
            Style::default().fg(app.theme.text_primary),
        ),
        Span::styled("█", Style::default().fg(app.theme.accent_primary)),
    ]));

    // Placeholder hint
    if app.goto_input.is_empty() {
        lines.push(Line::from(Span::styled(
            "  tomorrow, next friday, mar 15, 2026-06-10...",
            Style::default().fg(app.theme.text_dim),
        )));
    }

    // Preview
    lines.push(Line::raw(""));
    match &app.goto_preview {
        Some(preview) => {
            lines.push(Line::from(vec![
                Span::styled("→ ", Style::default().fg(app.theme.accent_success)),
                Span::styled(
                    preview.clone(),
                    Style::default().fg(app.theme.accent_success),
                ),
            ]));
        }
        None if !app.goto_input.is_empty() => {
            lines.push(Line::from(Span::styled(
                "  Could not parse date",
                Style::default().fg(app.theme.text_dim),
            )));
        }
        _ => {}
    }

    frame.render_widget(Paragraph::new(lines), inner);
}
