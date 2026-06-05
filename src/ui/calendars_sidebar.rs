use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::app_state::AppState;
use crate::state::focus::FocusContext;

pub const CALENDARS_SIDEBAR_WIDTH: u16 = 22;

pub fn render_calendars_sidebar(app: &AppState, frame: &mut Frame, area: Rect) {
    if !app.calendar_sidebar_visible {
        return;
    }

    let is_focused = app.focus == FocusContext::Calendars;
    let border_style = if is_focused {
        Style::default().fg(app.theme.accent_primary)
    } else {
        Style::default().fg(app.theme.text_dim)
    };

    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_type(BorderType::Plain)
        .border_style(border_style)
        .title(Span::styled("Calendars", Style::default().fg(app.theme.text_dim)));

    let inner = block.inner(area);
    app.calendar_sidebar_height.set(inner.height);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    let mut current_account = String::new();

    for (idx, cal) in app.calendars.iter().enumerate() {
        // Account separator
        if cal.account_email != current_account {
            if !current_account.is_empty() {
                lines.push(Line::raw(""));
            }
            let account_label = if cal.account_email.is_empty() {
                "CalDAV".to_string()
            } else {
                cal.account_email.clone()
            };
            lines.push(Line::from(Span::styled(
                format!(" {}", truncate(&account_label, inner.width as usize - 2)),
                Style::default()
                    .fg(app.theme.text_dim)
                    .add_modifier(Modifier::BOLD),
            )));
            current_account = cal.account_email.clone();
        }

        let is_selected = idx == app.selected_calendar_index;
        let is_enabled = !app.disabled_calendars.contains(&cal.key);
        let cal_color = cal.color.unwrap_or_else(|| app.theme.calendar_color(&cal.key));

        let check = if is_enabled { "●" } else { "○" };
        let name = truncate(&cal.display_name, inner.width as usize - 4);
        let label = format!("  {} {}", check, name);

        let style = if is_selected && is_focused {
            Style::default()
                .bg(app.theme.selection_bg)
                .fg(app.theme.selection_text)
        } else if is_enabled {
            Style::default().fg(cal_color)
        } else {
            Style::default().fg(app.theme.text_dim)
        };

        lines.push(Line::from(Span::styled(label, style)));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            " No calendars",
            Style::default().fg(app.theme.text_dim),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default())
            .scroll((app.calendar_sidebar_scroll as u16, 0)),
        inner,
    );
}

fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
