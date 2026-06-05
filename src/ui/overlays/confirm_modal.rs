use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::state::app_state::AppState;
use crate::state::overlay::RecurrenceScope;
use crate::ui::layout_helpers::centered_fixed;

pub fn render_confirm_modal(app: &AppState, frame: &mut Frame, area: Rect, is_delete: bool) {
    let is_rsvp = app.pending_rsvp_status.is_some();
    let height = if is_rsvp { 8 } else { 9 };
    let rect = centered_fixed(48, height, area);
    frame.render_widget(Clear, rect);

    let title = if is_rsvp {
        " RSVP recurring event "
    } else if is_delete {
        " Delete recurring event "
    } else {
        " Edit recurring event "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.modal_border))
        .title(Span::styled(
            title,
            Style::default()
                .fg(app.theme.text_primary)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let scope = &app.recurrence_scope;

    let mut lines = vec![
        Line::from(Span::styled(
            "Apply to:",
            Style::default().fg(app.theme.text_dim),
        )),
        Line::raw(""),
        scope_line(app, scope, RecurrenceScope::This, "This event only"),
        scope_line(app, scope, RecurrenceScope::Following, "This and all future"),
    ];

    if !is_rsvp {
        lines.push(scope_line(app, scope, RecurrenceScope::All, "All in series"));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("Enter", Style::default().fg(app.theme.accent_primary)),
        Span::styled(":continue  ", Style::default().fg(app.theme.text_dim)),
        Span::styled("Esc", Style::default().fg(app.theme.accent_primary)),
        Span::styled(":cancel", Style::default().fg(app.theme.text_dim)),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);
}

fn scope_line<'a>(
    app: &AppState,
    current: &RecurrenceScope,
    option: RecurrenceScope,
    label: &'a str,
) -> Line<'a> {
    let selected = *current == option;
    let radio = if selected { "◉ " } else { "○ " };
    let style = if selected {
        Style::default()
            .fg(app.theme.accent_primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.theme.text_secondary)
    };
    Line::from(vec![
        Span::styled(radio, style),
        Span::styled(label, style),
    ])
}
