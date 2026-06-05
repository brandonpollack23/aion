use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::domain::time::{format_day_short, format_time, get_event_start};
use crate::state::app_state::AppState;

pub fn render_search_view(app: &AppState, frame: &mut Frame, area: Rect) {
    // Floating panel anchored to top-left, below header
    let width = area.width.min(60);
    let max_results = 10usize;
    let height = (max_results as u16 + 4).min(area.height);

    let rect = Rect {
        x: area.x,
        y: area.y,
        width,
        height,
    };
    frame.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent_primary))
        .title(Span::styled(
            " Search ",
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    if inner.height == 0 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // Query line
    lines.push(Line::from(vec![
        Span::styled("/", Style::default().fg(app.theme.accent_primary)),
        Span::styled(
            app.search_query.clone(),
            Style::default().fg(app.theme.text_primary),
        ),
        Span::styled("█", Style::default().fg(app.theme.accent_primary)),
    ]));
    lines.push(Line::raw(""));

    if app.search_results.is_empty() && !app.search_query.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No results",
            Style::default().fg(app.theme.text_dim),
        )));
    } else {
        for (i, event) in app.search_results.iter().take(max_results).enumerate() {
            let selected = i == app.search_selected_index;
            let prefix = if selected { "▶ " } else { "  " };

            let date_str = get_event_start(event, &app.timezone)
                .map(|dt| format!("{} {}", format_day_short(dt.date_naive()), format_time(&dt)))
                .or_else(|| {
                    event.start.date.as_ref().and_then(|d| {
                        chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
                            .ok()
                            .map(format_day_short)
                    })
                })
                .unwrap_or_default();

            let style = if selected {
                Style::default()
                    .fg(app.theme.accent_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.text_secondary)
            };

            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(
                    format!("{:<16}", date_str),
                    Style::default().fg(app.theme.text_dim),
                ),
                Span::styled(event.display_title().to_string(), style),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}
