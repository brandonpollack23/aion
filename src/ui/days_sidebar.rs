use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::domain::time::{format_day_short, is_today, is_weekend};
use crate::state::app_state::AppState;
use crate::state::focus::FocusContext;

pub const DAYS_SIDEBAR_WIDTH: u16 = 10;

pub fn render_days_sidebar(app: &AppState, frame: &mut Frame, area: Rect) {
    let is_focused = app.focus == FocusContext::Days;
    let visible_lines = area.height as usize;
    let days = app.days_list(visible_lines);

    let lines: Vec<Line> = days
        .iter()
        .map(|&day| {
            let is_selected = day == app.selected_day;
            let label = format_day_short(day);
            let padded = format!(" {:<8}", label);

            let style = if is_selected && is_focused {
                Style::default()
                    .bg(app.theme.selection_bg)
                    .fg(app.theme.selection_text)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .fg(app.theme.accent_primary)
                    .add_modifier(Modifier::BOLD)
            } else if is_today(day) {
                Style::default().fg(app.theme.accent_success)
            } else if is_weekend(day) {
                Style::default().fg(app.theme.text_weekend)
            } else {
                Style::default().fg(app.theme.text_secondary)
            };

            let prefix = if is_selected && is_focused {
                "▶"
            } else if is_selected {
                "▸"
            } else {
                " "
            };
            Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(padded, style),
            ])
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(app.theme.text_dim)),
        area,
    );
}
