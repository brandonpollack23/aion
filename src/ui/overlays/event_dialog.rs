use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::state::app_state::AppState;
use crate::state::dialog::{
    DialogState, EVENT_TYPE_LABELS, F_ALL_DAY, F_END_DATE, F_END_TIME, F_EVENT_TYPE, F_LOCATION,
    F_NOTES, F_START_DATE, F_START_TIME, F_TITLE, F_WHEN,
};
use crate::ui::layout_helpers::centered_fixed;

const DIALOG_WIDTH: u16 = 62;
const LABEL_W: usize = 9;

pub fn render_event_dialog(app: &AppState, frame: &mut Frame, area: Rect) {
    let ds = match app.dialog_state.as_ref() {
        Some(s) => s,
        None => return,
    };

    let dialog_height = compute_height(ds, area.height);
    let rect = centered_fixed(DIALOG_WIDTH, dialog_height, area);

    frame.render_widget(Clear, rect);

    let title = if ds.is_edit {
        " Edit Event "
    } else {
        " New Event "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent_primary))
        .title(Span::styled(
            title,
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            " Tab:next  Shift-Tab:prev  Ctrl+S:save  Esc:cancel ",
            Style::default().fg(app.theme.text_dim),
        ));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    if inner.height == 0 {
        return;
    }

    let lines = build_form_lines(app, ds, inner.width);
    frame.render_widget(Paragraph::new(lines), inner);
}

fn compute_height(ds: &DialogState, max: u16) -> u16 {
    let mut rows: u16 = 2; // borders
    rows += 1; // title
    rows += 1; // type + all-day
    rows += 1; // when
    if ds.when_preview.is_some() {
        rows += 1;
    }
    rows += 1; // start
    if !ds.is_all_day {
        rows += 0; // start time on same line
    }
    rows += 1; // end
    rows += 1; // location
    rows += 1; // notes
    rows += 1; // empty row
    rows += 1; // footer hint
    rows.min(max)
}

fn field_style(app: &AppState, active: bool) -> Style {
    if active {
        Style::default()
            .fg(app.theme.text_primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.theme.text_secondary)
    }
}

fn label_span(label: &'static str, app: &AppState) -> Span<'static> {
    Span::styled(
        format!("{:<label_w$} ", label, label_w = LABEL_W),
        Style::default().fg(app.theme.text_dim),
    )
}

fn text_field_spans<'a>(
    app: &'a AppState,
    text: &'a str,
    active: bool,
    placeholder: &'static str,
) -> Vec<Span<'a>> {
    if text.is_empty() && !active {
        vec![Span::styled(
            placeholder,
            Style::default().fg(app.theme.text_dim),
        )]
    } else {
        let mut spans = vec![Span::styled(text, field_style(app, active))];
        if active {
            spans.push(Span::styled(
                "█",
                Style::default().fg(app.theme.accent_primary),
            ));
        }
        spans
    }
}

fn build_form_lines<'a>(app: &'a AppState, ds: &'a DialogState, width: u16) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    // Title
    {
        let active = ds.active_field == F_TITLE;
        let mut spans = vec![label_span("title", app)];
        spans.extend(text_field_spans(app, &ds.title, active, "(no title)"));
        lines.push(Line::from(spans));
    }

    // Type + All-day on one line
    {
        let type_active = ds.active_field == F_EVENT_TYPE;
        let allday_active = ds.active_field == F_ALL_DAY;
        let type_label = EVENT_TYPE_LABELS[ds.event_type_idx];

        let type_style = if type_active {
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.text_secondary)
        };
        let allday_style = if allday_active {
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.text_secondary)
        };
        let checkbox = if ds.is_all_day { "[x]" } else { "[ ]" };

        lines.push(Line::from(vec![
            label_span("type", app),
            Span::styled(format!("{:<16}", type_label), type_style),
            Span::styled(checkbox, allday_style),
            Span::styled(" all-day", Style::default().fg(app.theme.text_dim)),
        ]));
    }

    // When (natural language)
    {
        let active = ds.active_field == F_WHEN;
        let mut spans = vec![label_span("when", app)];
        spans.extend(text_field_spans(
            app,
            &ds.when_input,
            active,
            "tomorrow 3pm, next fri, mar 15...",
        ));
        lines.push(Line::from(spans));

        if let Some(ref preview) = ds.when_preview {
            lines.push(Line::from(vec![
                Span::raw(format!("{:<label_w$} ", "", label_w = LABEL_W)),
                Span::styled("→ ", Style::default().fg(app.theme.accent_success)),
                Span::styled(
                    preview.clone(),
                    Style::default().fg(app.theme.accent_success),
                ),
                Span::styled(
                    "  (Enter to apply)",
                    Style::default().fg(app.theme.text_dim),
                ),
            ]));
        }
    }

    // Start date [+ time]
    {
        let date_active = ds.active_field == F_START_DATE;
        let time_active = ds.active_field == F_START_TIME;

        let mut spans = vec![label_span("start", app)];
        spans.extend(text_field_spans(
            app,
            &ds.start_date,
            date_active,
            "YYYY-MM-DD",
        ));

        if !ds.is_all_day {
            spans.push(Span::styled("  ", Style::default()));
            spans.extend(text_field_spans(app, &ds.start_time, time_active, "HH:MM"));
        }
        lines.push(Line::from(spans));
    }

    // End date [+ time]
    {
        let date_active = ds.active_field == F_END_DATE;
        let time_active = ds.active_field == F_END_TIME;

        let mut spans = vec![label_span("end", app)];
        spans.extend(text_field_spans(
            app,
            &ds.end_date,
            date_active,
            "YYYY-MM-DD",
        ));

        if !ds.is_all_day {
            spans.push(Span::styled("  ", Style::default()));
            spans.extend(text_field_spans(app, &ds.end_time, time_active, "HH:MM"));
        }
        lines.push(Line::from(spans));
    }

    // Location
    {
        let active = ds.active_field == F_LOCATION;
        let mut spans = vec![label_span("location", app)];
        spans.extend(text_field_spans(app, &ds.location, active, "Add location"));
        lines.push(Line::from(spans));
    }

    // Notes
    {
        let active = ds.active_field == F_NOTES;
        let mut spans = vec![label_span("notes", app)];
        spans.extend(text_field_spans(app, &ds.notes, active, "Add notes"));
        lines.push(Line::from(spans));
    }

    // Spacer
    lines.push(Line::raw(""));

    // Action hint
    let save_label = if ds.is_edit { "save" } else { "create" };
    let active_action = Style::default()
        .fg(app.theme.accent_primary)
        .add_modifier(Modifier::BOLD);
    lines.push(Line::from(vec![
        Span::styled(
            format!("{:>w$}", "", w = (width as usize).saturating_sub(20)),
            Style::default(),
        ),
        Span::styled("[cancel]", Style::default().fg(app.theme.text_dim)),
        Span::styled("  ", Style::default()),
        Span::styled(format!("[{}]", save_label), active_action),
    ]));

    lines
}
