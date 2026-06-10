use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::domain::event::{CalEvent, EventType, ReminderMethod, ResponseStatus};
use crate::domain::time::{format_day_header, format_time, get_event_end, get_event_start};
use crate::state::app_state::AppState;
use crate::ui::layout_helpers::right_panel;

pub const PANEL_WIDTH: u16 = 50;
const LABEL_WIDTH: u16 = 11;

pub fn render_details_panel(app: &AppState, frame: &mut Frame, area: Rect) {
    let event = match app
        .selected_event_id
        .as_ref()
        .and_then(|id| app.events.get(id))
    {
        Some(e) => e,
        None => return,
    };

    let panel_rect = right_panel(PANEL_WIDTH, area);
    frame.render_widget(Clear, panel_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent_primary))
        .title(Span::styled(
            " Event Details ",
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            " esc:close  e:edit  D:delete  y/n/m:rsvp ",
            Style::default().fg(app.theme.text_dim),
        ));

    let inner = block.inner(panel_rect);
    frame.render_widget(block, panel_rect);

    if inner.height == 0 {
        return;
    }

    let lines = build_detail_lines(app, event);

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .style(Style::default()),
        inner,
    );
}

fn build_detail_lines<'a>(app: &'a AppState, event: &'a CalEvent) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();
    let tz = &app.timezone;

    // Title
    lines.push(row_line(
        app,
        "title",
        vec![Span::styled(
            event.display_title().to_string(),
            Style::default()
                .fg(app.theme.text_primary)
                .add_modifier(Modifier::BOLD),
        )],
    ));

    // Account + calendar
    if let Some(ref email) = event.account_email {
        let color = app.theme.calendar_color(&event.calendar_key());
        lines.push(row_line(
            app,
            "account",
            vec![
                Span::styled("● ", Style::default().fg(color)),
                Span::styled(email.clone(), Style::default().fg(app.theme.text_secondary)),
            ],
        ));
    }
    if let (Some(ref cal_id), Some(ref email)) = (&event.calendar_id, &event.account_email) {
        if cal_id != email {
            lines.push(row_line(
                app,
                "calendar",
                vec![Span::styled(
                    cal_id.clone(),
                    Style::default().fg(app.theme.text_secondary),
                )],
            ));
        }
    }

    // Organizer
    if let Some(ref org) = event.organizer {
        let label = if org.is_self == Some(true) {
            format!(
                "{} (you)",
                format_email(org.email.clone(), org.display_name.clone())
            )
        } else {
            format_email(org.email.clone(), org.display_name.clone())
        };
        lines.push(row_line(
            app,
            "organizer",
            vec![Span::styled(
                label,
                Style::default().fg(app.theme.text_secondary),
            )],
        ));
    }

    // Event type
    let type_label = match event.event_type.as_ref() {
        Some(EventType::OutOfOffice) => "Out of office",
        Some(EventType::FocusTime) => "Focus time",
        Some(EventType::Birthday) => "Birthday",
        _ => "Event",
    };
    let type_color = match event.event_type.as_ref() {
        Some(EventType::OutOfOffice) => app.theme.event_out_of_office,
        Some(EventType::FocusTime) => app.theme.event_focus_time,
        Some(EventType::Birthday) => app.theme.event_birthday,
        _ => app.theme.event_default,
    };
    let recurring_suffix = if event.is_recurring() { "  ↻" } else { "" };
    lines.push(row_line(
        app,
        "type",
        vec![
            Span::styled(type_label.to_string(), Style::default().fg(type_color)),
            Span::styled(
                recurring_suffix.to_string(),
                Style::default().fg(app.theme.text_dim),
            ),
        ],
    ));

    // Date and time
    let start_dt = get_event_start(event, tz);
    let end_dt = get_event_end(event, tz);

    if event.is_all_day() {
        if let Some(ref d) = event.start.date {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d") {
                lines.push(row_line(
                    app,
                    "date",
                    vec![Span::styled(
                        format_day_header(date),
                        Style::default().fg(app.theme.text_primary),
                    )],
                ));
            }
        }
        lines.push(row_line(
            app,
            "time",
            vec![Span::styled(
                "All day".to_string(),
                Style::default().fg(app.theme.text_secondary),
            )],
        ));
    } else if let (Some(ref s), Some(ref e)) = (start_dt, end_dt) {
        lines.push(row_line(
            app,
            "date",
            vec![Span::styled(
                format_day_header(s.date_naive()),
                Style::default().fg(app.theme.text_primary),
            )],
        ));
        let time_range = format!("{} – {}", format_time(s), format_time(e));
        lines.push(row_line(
            app,
            "time",
            vec![Span::styled(
                time_range,
                Style::default().fg(app.theme.text_secondary),
            )],
        ));
    }

    // Timezone
    let tz_display = tz.split('/').last().unwrap_or(tz).replace('_', " ");
    lines.push(row_line(
        app,
        "tz",
        vec![Span::styled(
            tz_display,
            Style::default().fg(app.theme.text_dim),
        )],
    ));

    // Recurrence
    if let Some(rrule) = event.parse_recurrence_rule() {
        lines.push(row_line(
            app,
            "repeats",
            vec![Span::styled(
                rrule,
                Style::default().fg(app.theme.text_secondary),
            )],
        ));
    }

    // RSVP status
    let self_status = event.self_response_status();
    let (status_icon, status_color) = match self_status {
        Some(ResponseStatus::Accepted) => ("✓ accepted", app.theme.status_accepted),
        Some(ResponseStatus::Declined) => ("✗ declined", app.theme.status_declined),
        Some(ResponseStatus::Tentative) => ("? maybe", app.theme.status_tentative),
        Some(ResponseStatus::NeedsAction) => ("○ awaiting", app.theme.status_needs_action),
        None => ("", app.theme.text_dim),
    };
    if !status_icon.is_empty() {
        lines.push(row_line(
            app,
            "going?",
            vec![
                Span::styled(status_icon.to_string(), Style::default().fg(status_color)),
                Span::styled(
                    "  y:yes  n:no  m:maybe".to_string(),
                    Style::default().fg(app.theme.text_dim),
                ),
            ],
        ));
    }

    // Location
    if let Some(ref loc) = event.location {
        lines.push(row_line(
            app,
            "where",
            vec![Span::styled(
                loc.clone(),
                Style::default().fg(app.theme.text_primary),
            )],
        ));
    }

    // Meeting link — prefer hangout_link, then scan location/description
    let extra_link = event
        .hangout_link
        .as_deref()
        .map(|s| s.to_string())
        .or_else(|| {
            let candidates = [
                event.location.as_deref().unwrap_or(""),
                event.description.as_deref().unwrap_or(""),
            ];
            for text in &candidates {
                if let Some(link) = extract_meeting_link(text) {
                    return Some(link);
                }
            }
            None
        });
    if let Some(ref link) = extra_link {
        let label = meeting_link_label(link);
        lines.push(row_line(
            app,
            label,
            vec![Span::styled(
                link.clone(),
                Style::default().fg(app.theme.accent_primary),
            )],
        ));
    }

    // Attendees
    if let Some(ref attendees) = event.attendees {
        if !attendees.is_empty() {
            lines.push(row_line(
                app,
                "guests",
                vec![Span::styled(
                    format!(
                        "{} attendee{}",
                        attendees.len(),
                        if attendees.len() == 1 { "" } else { "s" }
                    ),
                    Style::default().fg(app.theme.text_dim),
                )],
            ));
            for attendee in attendees.iter().take(8) {
                let (icon, color) = match attendee.response_status.as_ref() {
                    Some(ResponseStatus::Accepted) => ("✓", app.theme.status_accepted),
                    Some(ResponseStatus::Declined) => ("✗", app.theme.status_declined),
                    Some(ResponseStatus::Tentative) => ("?", app.theme.status_tentative),
                    _ => ("○", app.theme.status_needs_action),
                };
                let name = format_email(attendee.email.clone(), attendee.display_name.clone());
                lines.push(row_line(
                    app,
                    "",
                    vec![
                        Span::styled(format!("{} ", icon), Style::default().fg(color)),
                        Span::styled(name, Style::default().fg(app.theme.text_secondary)),
                    ],
                ));
            }
            if attendees.len() > 8 {
                lines.push(row_line(
                    app,
                    "",
                    vec![Span::styled(
                        format!("  +{} more", attendees.len() - 8),
                        Style::default().fg(app.theme.text_dim),
                    )],
                ));
            }
        }
    }

    // Reminders
    if let Some(ref rem) = event.reminders {
        if let Some(ref overrides) = rem.overrides {
            for r in overrides {
                let method = match r.method {
                    ReminderMethod::Email => "Email",
                    ReminderMethod::Popup => "Notif",
                };
                let text = format_reminder_text(r.minutes, method);
                lines.push(row_line(
                    app,
                    "remind",
                    vec![Span::styled(
                        text,
                        Style::default().fg(app.theme.text_secondary),
                    )],
                ));
            }
        }
    }

    // Description
    if let Some(ref desc) = event.description {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "notes",
            Style::default().fg(app.theme.text_dim),
        )));
        // Strip basic HTML tags
        let plain = strip_html(desc);
        for text_line in plain.lines().take(20) {
            lines.push(Line::from(Span::styled(
                text_line.to_string(),
                Style::default().fg(app.theme.text_secondary),
            )));
        }
    }

    lines
}

fn row_line<'a>(app: &'a AppState, label: &'a str, content: Vec<Span<'a>>) -> Line<'a> {
    let mut spans = vec![Span::styled(
        format!("{:<label_w$} ", label, label_w = LABEL_WIDTH as usize),
        Style::default().fg(app.theme.text_dim),
    )];
    spans.extend(content);
    Line::from(spans)
}

fn format_email(email: String, display_name: Option<String>) -> String {
    if let Some(name) = display_name {
        if !name.is_empty() && name != email {
            return format!("{} <{}>", name, email);
        }
    }
    email
}

fn meeting_link_label(url: &str) -> &'static str {
    if url.contains("meet.google.com") {
        return "meet";
    }
    if url.contains("zoom.us") {
        return "zoom";
    }
    if url.contains("teams.microsoft.com") {
        return "teams";
    }
    if url.contains("webex.com") {
        return "webex";
    }
    if url.contains("whereby.com") {
        return "whereby";
    }
    if url.contains("jit.si") {
        return "jitsi";
    }
    "link"
}

const MEETING_DOMAINS: &[&str] = &[
    "meet.google.com",
    "zoom.us",
    "teams.microsoft.com",
    "webex.com",
    "whereby.com",
    "jit.si",
    "gotomeeting.com",
    "bluejeans.com",
    "chime.aws",
];

fn extract_meeting_link(text: &str) -> Option<String> {
    // Find the first https:// URL containing a known meeting domain
    let mut pos = 0;
    while let Some(start) = text[pos..].find("https://") {
        let abs = pos + start;
        let rest = &text[abs..];
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '<' || c == '>' || c == '"' || c == '\'')
            .unwrap_or(rest.len());
        let url = &rest[..end];
        if MEETING_DOMAINS.iter().any(|d| url.contains(d)) {
            return Some(url.to_string());
        }
        pos = abs + 8; // move past "https://"
    }
    None
}

fn format_reminder_text(minutes: i64, method: &str) -> String {
    if minutes == 0 {
        return format!("{} at event time", method);
    }
    if minutes < 60 {
        return format!("{} {} min before", method, minutes);
    }
    if minutes < 1440 {
        let h = minutes / 60;
        return format!("{} {}hr before", method, h);
    }
    let d = minutes / 1440;
    format!("{} {}d before", method, d)
}

fn strip_html(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    // Normalize whitespace a bit
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
}
