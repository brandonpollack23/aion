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
    // Render `:query` into the status bar area (full width)
    let line = Line::from(vec![
        Span::styled(":", Style::default().fg(app.theme.accent_primary)),
        Span::styled(
            app.command_input.clone(),
            Style::default().fg(app.theme.text_primary),
        ),
        Span::styled("█", Style::default().fg(app.theme.accent_primary)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

pub fn render_command_completions(app: &AppState, frame: &mut Frame, area: Rect) {
    let completions = filter_commands(&app.command_input);
    if completions.is_empty() {
        return;
    }

    let count = completions.len().min(MAX_COMPLETIONS) as u16;
    let width = area.width.min(56);
    let height = count + 2; // border top + bottom

    if area.height < height + 1 {
        return;
    }

    // Float above the status bar
    let rect = Rect {
        x: area.x,
        y: area.y.saturating_sub(height),
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
    for (i, cmd) in completions.iter().take(MAX_COMPLETIONS).enumerate() {
        let selected = i == app.command_selected_index;
        let style = if selected {
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.text_secondary)
        };
        let prefix = if selected { "▶ " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(format!("{:<20}", cmd.name), style),
            Span::styled(cmd.description, Style::default().fg(app.theme.text_dim)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}
