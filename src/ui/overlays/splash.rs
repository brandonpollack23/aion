use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::state::app_state::AppState;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn render_splash(app: &AppState, frame: &mut Frame, area: Rect) {
    // Centered box: 40 wide, 7 tall
    let w = 40u16.min(area.width);
    let h = 7u16.min(area.height);
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    let rect = Rect::new(x, y, w, h);

    frame.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent_primary))
        .title(Span::styled(
            " aion ",
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let spinner = SPINNER_FRAMES[app.splash_tick % SPINNER_FRAMES.len()];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} ", spinner),
                Style::default().fg(app.theme.accent_primary),
            ),
            Span::styled(
                "Loading events…",
                Style::default().fg(app.theme.text_dim),
            ),
        ]))
        .alignment(Alignment::Center),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Ctrl+C to quit",
            Style::default().fg(app.theme.text_dim),
        )))
        .alignment(Alignment::Center),
        chunks[2],
    );
}
