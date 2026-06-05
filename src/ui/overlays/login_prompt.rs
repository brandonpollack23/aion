use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::state::app_state::AppState;
use crate::ui::layout_helpers::centered_fixed;

pub fn render_login_prompt(app: &AppState, frame: &mut Frame, area: Rect) {
    let rect = centered_fixed(52, 7, area);
    frame.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.modal_border))
        .title(Span::styled(
            " Google Calendar ",
            Style::default()
                .fg(app.theme.text_primary)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let lines = vec![
        Line::from(Span::styled(
            "Credentials are configured but you are not logged in.",
            Style::default().fg(app.theme.text_secondary),
        )),
        Line::raw(""),
        Line::from(Span::styled(
            "Would you like to log in to Google Calendar now?",
            Style::default().fg(app.theme.text_primary),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::styled("y", Style::default().fg(app.theme.accent_success).add_modifier(Modifier::BOLD)),
            Span::styled(":login  ", Style::default().fg(app.theme.text_dim)),
            Span::styled("n", Style::default().fg(app.theme.accent_error).add_modifier(Modifier::BOLD)),
            Span::styled("/", Style::default().fg(app.theme.text_dim)),
            Span::styled("Esc", Style::default().fg(app.theme.accent_primary)),
            Span::styled(":dismiss", Style::default().fg(app.theme.text_dim)),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}
