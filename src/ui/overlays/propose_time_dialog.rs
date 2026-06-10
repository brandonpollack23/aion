use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::state::app_state::AppState;
use crate::state::propose_time::{
    PT_FIELD_END_DATE, PT_FIELD_END_TIME, PT_FIELD_MESSAGE, PT_FIELD_START_DATE,
    PT_FIELD_START_TIME, PT_FIELD_WHEN,
};

const DIALOG_WIDTH: u16 = 52;
const LABEL_W: u16 = 9;

pub fn render_propose_time_dialog(app: &AppState, frame: &mut Frame, area: Rect) {
    let pt = match app.propose_time.as_ref() {
        Some(s) => s,
        None => return,
    };

    let width = DIALOG_WIDTH.min(area.width);
    let height = 16u16.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let rect = Rect {
        x,
        y,
        width,
        height,
    };

    frame.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Propose New Time ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // event title
            Constraint::Length(1), // organizer
            Constraint::Length(1), // spacer
            Constraint::Length(2), // when + preview
            Constraint::Length(1), // start date+time
            Constraint::Length(1), // end date+time
            Constraint::Length(1), // spacer
            Constraint::Length(1), // message
            Constraint::Fill(1),   // spacer
            Constraint::Length(1), // footer
        ])
        .split(inner);
    let when_interpretation = pt.when_interpretation();
    let previewing_when = when_interpretation.is_some();
    let display_start_date = when_interpretation
        .as_ref()
        .map_or(pt.start_date.as_str(), |preview| {
            preview.start_date.as_str()
        });
    let display_start_time = when_interpretation
        .as_ref()
        .map_or(pt.start_time.as_str(), |preview| {
            preview.start_time.as_str()
        });
    let display_end_date = when_interpretation
        .as_ref()
        .map_or(pt.end_date.as_str(), |preview| preview.end_date.as_str());
    let display_end_time = when_interpretation
        .as_ref()
        .map_or(pt.end_time.as_str(), |preview| preview.end_time.as_str());

    // Event title
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("for: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&pt.event_title, Style::default().fg(Color::White)),
        ])),
        rows[0],
    );

    // Organizer
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("org:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(&pt.organizer, Style::default().fg(Color::DarkGray)),
        ])),
        rows[1],
    );

    // When field
    let when_active = pt.active_field == PT_FIELD_WHEN;
    let when_style = if when_active {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let when_line = Line::from(vec![
        Span::styled(
            "when:    ",
            Style::default().fg(if when_active {
                Color::Cyan
            } else {
                Color::DarkGray
            }),
        ),
        Span::styled(&pt.when_input, when_style),
        if when_active {
            Span::styled("█", Style::default().fg(Color::Cyan))
        } else {
            Span::raw("")
        },
    ]);

    let when_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(rows[3]);
    frame.render_widget(Paragraph::new(when_line), when_rows[0]);

    if let Some(ref preview) = pt.when_preview {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("         "),
                Span::styled(
                    format!("● {} Enter", preview),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ])),
            when_rows[1],
        );
    }

    // Start row
    render_date_time_row(
        frame,
        rows[4],
        "start:   ",
        display_start_date,
        Some(display_start_time),
        pt.active_field == PT_FIELD_START_DATE || pt.active_field == PT_FIELD_START_TIME,
        pt.active_field == PT_FIELD_START_DATE,
        pt.active_field == PT_FIELD_START_TIME,
        previewing_when,
    );

    // End row
    render_date_time_row(
        frame,
        rows[5],
        "end:     ",
        display_end_date,
        Some(display_end_time),
        pt.active_field == PT_FIELD_END_DATE || pt.active_field == PT_FIELD_END_TIME,
        pt.active_field == PT_FIELD_END_DATE,
        pt.active_field == PT_FIELD_END_TIME,
        previewing_when,
    );

    // Message
    let msg_active = pt.active_field == PT_FIELD_MESSAGE;
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "message: ",
                Style::default().fg(if msg_active {
                    Color::Cyan
                } else {
                    Color::DarkGray
                }),
            ),
            Span::styled(
                &pt.message,
                if msg_active {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                },
            ),
            if msg_active {
                Span::styled("█", Style::default().fg(Color::Cyan))
            } else {
                Span::raw("")
            },
        ])),
        rows[7],
    );

    // Footer
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::DarkGray)),
            Span::raw(":next  "),
            Span::styled("^S", Style::default().fg(Color::DarkGray)),
            Span::raw(":send  "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(":cancel"),
        ])),
        rows[9],
    );
}

fn render_date_time_row(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    date: &str,
    time: Option<&str>,
    _row_active: bool,
    date_active: bool,
    time_active: bool,
    previewing_when: bool,
) {
    let preview_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let date_style = if date_active {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else if previewing_when {
        preview_style
    } else {
        Style::default().fg(Color::Gray)
    };
    let time_style = if time_active {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else if previewing_when {
        preview_style
    } else {
        Style::default().fg(Color::Gray)
    };

    let mut spans = vec![
        Span::styled(
            label,
            Style::default().fg(if date_active || time_active {
                Color::Cyan
            } else {
                Color::DarkGray
            }),
        ),
        Span::styled(date, date_style),
        if date_active {
            Span::styled("█", Style::default().fg(Color::Cyan))
        } else {
            Span::raw("")
        },
    ];

    if let Some(t) = time {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(t, time_style));
        if time_active {
            spans.push(Span::styled("█", Style::default().fg(Color::Cyan)));
        }
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
