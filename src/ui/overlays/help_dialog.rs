use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::keybinds::help::{
    CALENDARS_BINDS, COMMAND_BINDS, DAYS_BINDS, DETAILS_BINDS, DIALOG_BINDS, GLOBAL_BINDS,
    MONTH_BINDS, SEARCH_BINDS, TIMELINE_BINDS,
};
use crate::state::app_state::AppState;
use crate::ui::layout_helpers::centered_fixed;

pub fn render_help_dialog(app: &AppState, frame: &mut Frame, area: Rect) {
    let width = 58u16.min(area.width);
    let height = (area.height * 4 / 5).max(10);
    let rect = centered_fixed(width, height, area);

    frame.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent_primary))
        .title(Span::styled(
            " Keyboard Shortcuts ",
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            " esc:close ",
            Style::default().fg(app.theme.text_dim),
        ));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    add_section(&mut lines, app, &GLOBAL_BINDS);
    add_section(&mut lines, app, &TIMELINE_BINDS);
    add_section(&mut lines, app, &MONTH_BINDS);
    add_section(&mut lines, app, &DAYS_BINDS);
    add_section(&mut lines, app, &CALENDARS_BINDS);
    add_section(&mut lines, app, &DETAILS_BINDS);
    add_section(&mut lines, app, &DIALOG_BINDS);
    add_section(&mut lines, app, &SEARCH_BINDS);
    add_section(&mut lines, app, &COMMAND_BINDS);

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn add_section(
    lines: &mut Vec<Line<'static>>,
    app: &AppState,
    section: &crate::keybinds::help::Section,
) {
    lines.push(Line::from(Span::styled(
        section.title,
        Style::default()
            .fg(app.theme.text_dim)
            .add_modifier(Modifier::BOLD),
    )));
    for bind in section.binds {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<14}", bind.display),
                Style::default().fg(app.theme.accent_primary),
            ),
            Span::styled(
                bind.description,
                Style::default().fg(app.theme.text_secondary),
            ),
        ]));
    }
    lines.push(Line::raw(""));
}
