use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::keybinds::commands::filter_commands;
use crate::state::app_state::AppState;

const MAX_COMPLETIONS: usize = 8;

pub fn render_command_input(app: &AppState, frame: &mut Frame, area: Rect) {
    let cursor = app.command_cursor.min(app.command_input.chars().count());
    let before: String = app.command_input.chars().take(cursor).collect();
    let at: String = app
        .command_input
        .chars()
        .nth(cursor)
        .map(|c| c.to_string())
        .unwrap_or_default();
    let after: String = app
        .command_input
        .chars()
        .skip(cursor + if at.is_empty() { 0 } else { 1 })
        .collect();

    let mut spans = vec![
        Span::styled(":", Style::default().fg(app.theme.accent_primary)),
        Span::styled(before, Style::default().fg(app.theme.text_primary)),
    ];
    if at.is_empty() {
        spans.push(Span::styled(
            "█",
            Style::default().fg(app.theme.accent_primary),
        ));
    } else {
        spans.push(Span::styled(
            at,
            Style::default()
                .fg(app.theme.statusbar_bg)
                .bg(app.theme.accent_primary),
        ));
        spans.push(Span::styled(
            after,
            Style::default().fg(app.theme.text_primary),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

pub fn render_command_completions(app: &AppState, frame: &mut Frame, area: Rect) {
    let query = app
        .command_completion_query
        .as_deref()
        .unwrap_or(&app.command_input);
    let completions = filter_commands(query);
    if completions.is_empty() {
        return;
    }

    let total = completions.len();
    let visible = total.min(MAX_COMPLETIONS);
    let width = area.width.min(56);
    let height = visible as u16 + 2; // border top + bottom

    if area.height < height + 1 {
        return;
    }

    // The item highlighted is the one just filled (one behind the cycling pointer)
    let selected = if total == 0 {
        0
    } else {
        (app.command_selected_index + total - 1) % total
    };

    // Scroll so selected stays within [buffer, visible - buffer - 1]
    const BUFFER: usize = 1;
    let scroll = if selected < BUFFER {
        0
    } else if selected + BUFFER + 1 > visible {
        (selected + BUFFER + 1 - visible).min(total - visible)
    } else {
        selected - BUFFER
    };

    // Float above the status bar (area is the full frame area; status bar is the last row)
    let rect = Rect {
        x: area.x,
        y: area.bottom().saturating_sub(height + 1),
        width,
        height,
    };

    frame.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.modal_border));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let mut lines: Vec<Line> = Vec::new();
    for (i, cmd) in completions.iter().enumerate().skip(scroll).take(visible) {
        let is_selected = i == selected;
        let style = if is_selected {
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.text_secondary)
        };
        let prefix = if is_selected { "▶ " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(format!("{:<20}", cmd.name), style),
            Span::styled(cmd.description, Style::default().fg(app.theme.text_dim)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}
