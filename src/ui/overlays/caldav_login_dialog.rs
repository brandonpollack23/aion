use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::state::app_state::AppState;
use crate::state::caldav_dialog::{
    CalDavDialogState, PRESET_NAMES, CD_FIELD_COUNT, CD_PASSWORD, CD_PASSWORD_CMD, CD_SERVER_URL,
};

const DIALOG_WIDTH: u16 = 66;
const DIALOG_HEIGHT: u16 = 22;
const LABEL_W: usize = 16;

pub fn render_caldav_login_dialog(app: &AppState, frame: &mut Frame, area: Rect) {
    let ds = match app.caldav_dialog.as_ref() {
        Some(s) => s,
        None => return,
    };

    let popup = centered_fixed(DIALOG_WIDTH, DIALOG_HEIGHT, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.accent_primary))
        .title(Span::styled(
            " Add CalDAV Account ",
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(app.theme.modal_bg));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let body_inner = inner.inner(Margin { horizontal: 1, vertical: 0 });

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Preset selector
            Constraint::Length(1), // spacer
            Constraint::Fill(1),   // fields
            Constraint::Length(1), // test result
            Constraint::Length(1), // footer
        ])
        .split(body_inner);

    render_preset_row(app, ds, frame, chunks[0]);
    render_fields(app, ds, frame, chunks[2]);
    render_test_result(app, ds, frame, chunks[3]);
    render_footer(app, ds, frame, chunks[4]);
}

fn render_preset_row(app: &AppState, ds: &CalDavDialogState, frame: &mut Frame, area: Rect) {
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled("Preset  ", Style::default().fg(app.theme.text_dim)));
    for (i, name) in PRESET_NAMES.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let style = if i == ds.preset_idx {
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(app.theme.text_secondary)
        };
        spans.push(Span::styled(format!("{}", i + 1), style));
        spans.push(Span::styled(format!(":{}", name), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

const FIELD_LABELS: &[&str] = &[
    "Name          ",
    "Email         ",
    "Server URL    ",
    "Username      ",
    "Password      ",
    "Password cmd  ",
];

fn render_fields(app: &AppState, ds: &CalDavDialogState, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    for i in 0..CD_FIELD_COUNT {
        let label = FIELD_LABELS[i];
        let value = &ds.fields[i];
        let is_active = i == ds.active_field;

        let display_value = if i == CD_PASSWORD && !value.is_empty() {
            "•".repeat(value.len())
        } else {
            value.clone()
        };

        let cursor_suffix = if is_active { "▌" } else { "" };

        let label_style = Style::default().fg(app.theme.text_dim);
        let value_style = if is_active {
            Style::default()
                .fg(app.theme.text_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.text_secondary)
        };

        lines.push(Line::from(vec![
            Span::styled(label, label_style),
            Span::styled(format!("{}{}", display_value, cursor_suffix), value_style),
        ]));

        // Hint line for some fields
        let hint = match i {
            CD_SERVER_URL => Some("e.g. https://caldav.icloud.com"),
            CD_PASSWORD_CMD => Some("e.g. pass show caldav/my-account"),
            _ => None,
        };
        if let Some(h) = hint {
            lines.push(Line::from(Span::styled(
                format!("{}  {}", " ".repeat(LABEL_W), h),
                Style::default().fg(app.theme.text_dim),
            )));
        } else {
            lines.push(Line::raw(""));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_test_result(app: &AppState, ds: &CalDavDialogState, frame: &mut Frame, area: Rect) {
    let line = if ds.testing {
        Line::from(Span::styled(
            "Testing connection…",
            Style::default().fg(app.theme.accent_warning),
        ))
    } else if let Some(ref result) = ds.test_result {
        match result {
            Ok(msg) => Line::from(Span::styled(
                format!("✓ {}", msg),
                Style::default().fg(app.theme.accent_success),
            )),
            Err(msg) => Line::from(Span::styled(
                format!("✗ {}", msg),
                Style::default().fg(app.theme.accent_error),
            )),
        }
    } else {
        Line::raw("")
    };
    frame.render_widget(Paragraph::new(line), area);
}

fn render_footer(app: &AppState, ds: &CalDavDialogState, frame: &mut Frame, area: Rect) {
    let hint = if ds.saving {
        "Saving…"
    } else {
        "Tab/Shift-Tab:navigate  1-6:preset  t:test  Ctrl-S:save  Esc:cancel"
    };
    let para = Paragraph::new(Span::styled(hint, Style::default().fg(app.theme.text_dim)));
    frame.render_widget(para, area);
}

fn centered_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}
