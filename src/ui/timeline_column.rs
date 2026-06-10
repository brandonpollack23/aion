use chrono::NaiveDate;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::domain::event::{CalEvent, EventType, ResponseStatus};
use crate::domain::layout::{minutes_to_slot, DayLayout, TimedEventLayout, SLOTS_PER_HOUR};
use crate::domain::time::{
    format_day_header, format_hour_label, format_time, get_event_end, get_event_start,
    get_now_minutes, is_today,
};
use crate::state::app_state::AppState;
use crate::state::focus::FocusContext;

pub const HOUR_LABEL_WIDTH: usize = 7;
const COLLAPSE_THRESHOLD: usize = 2;

pub fn render_timeline_column(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    day: NaiveDate,
    col_idx: usize,
    show_hour_labels: bool,
    max_all_day_count: usize,
    all_day_lines: usize,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let is_timeline_focused = app.focus == FocusContext::Timeline;
    let is_col_focused = is_timeline_focused && app.focused_column == col_idx;
    let today = is_today(day);
    let now_minutes = if today { get_now_minutes() } else { -1 };
    let now_slot = if now_minutes >= 0 {
        minutes_to_slot(now_minutes)
    } else {
        -1
    };

    let layout = app.day_layout(day);

    // Split area into: header (1), all_day (all_day_lines), timeline (rest)
    let header_y = area.y;
    let all_day_y = area.y + 1;
    let timeline_y = area.y + 1 + all_day_lines as u16;
    let timeline_height = area.height.saturating_sub(1 + all_day_lines as u16) as usize;

    // === Header row ===
    render_column_header(
        app,
        frame,
        area.x,
        header_y,
        area.width,
        day,
        is_col_focused,
        today,
        show_hour_labels,
    );

    // === All-day section ===
    if all_day_lines > 0 {
        render_all_day_section(
            app,
            frame,
            Rect::new(area.x, all_day_y, area.width, all_day_lines as u16),
            &layout,
            max_all_day_count,
            show_hour_labels,
        );
    }

    // === Timeline grid ===
    if timeline_height > 0 {
        let timeline_rect = Rect::new(area.x, timeline_y, area.width, timeline_height as u16);
        render_timeline_grid(
            app,
            frame,
            timeline_rect,
            &layout,
            day,
            now_slot,
            show_hour_labels,
            timeline_height,
        );
    }
}

fn render_column_header(
    app: &AppState,
    frame: &mut Frame,
    x: u16,
    y: u16,
    width: u16,
    day: NaiveDate,
    is_focused: bool,
    today: bool,
    show_hour_labels: bool,
) {
    let label_width = if show_hour_labels {
        HOUR_LABEL_WIDTH as u16
    } else {
        0
    };
    let text_x = x + label_width;
    let text_width = width.saturating_sub(label_width);
    if text_width == 0 {
        return;
    }

    let header_text = format_day_header(day);
    let today_suffix = if today { "  today" } else { "" };

    let prefix = if is_focused { "▶ " } else { "  " };
    let header_color = if is_focused {
        app.theme.accent_primary
    } else {
        app.theme.text_dim
    };
    let today_color = app.theme.accent_success;

    let full = format!("{}{}{}", prefix, header_text, today_suffix);
    let truncated = truncate_str(&full, text_width as usize);

    // Build spans: prefix + header + today
    let header_part = format!(
        "{}{}",
        prefix,
        &header_text[..header_text.len().min(text_width as usize - prefix.len())]
    );
    let line = if today && header_part.len() + today_suffix.len() < text_width as usize {
        Line::from(vec![
            Span::styled(
                format!("{}{}", prefix, header_text),
                if is_focused {
                    Style::default()
                        .fg(header_color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(header_color)
                },
            ),
            Span::styled(today_suffix, Style::default().fg(today_color)),
        ])
    } else {
        Line::from(Span::styled(
            truncated,
            if is_focused {
                Style::default()
                    .fg(header_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(header_color)
            },
        ))
    };

    let rect = Rect::new(text_x, y, text_width, 1);
    frame.render_widget(Paragraph::new(line), rect);
}

fn render_all_day_section(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    layout: &DayLayout,
    max_all_day_count: usize,
    show_hour_labels: bool,
) {
    let events = &layout.all_day_events;
    let should_collapse = max_all_day_count > COLLAPSE_THRESHOLD && !app.all_day_expanded;
    let visible_count = if should_collapse {
        COLLAPSE_THRESHOLD
    } else {
        max_all_day_count
    };

    for row in 0..area.height as usize {
        let y = area.y + row as u16;
        if row < visible_count {
            let event = events.get(row);
            render_all_day_event_row(
                app,
                frame,
                area.x,
                y,
                area.width,
                event,
                show_hour_labels,
                row == 0,
                &layout.all_day_events,
            );
        } else {
            // Collapse row or empty row
            render_all_day_empty_row(
                app,
                frame,
                area.x,
                y,
                area.width,
                show_hour_labels,
                events,
                should_collapse,
                visible_count,
            );
        }
    }
}

fn render_all_day_event_row(
    app: &AppState,
    frame: &mut Frame,
    x: u16,
    y: u16,
    width: u16,
    event: Option<&CalEvent>,
    show_hour_labels: bool,
    is_first: bool,
    all_events: &[CalEvent],
) {
    let label_w = if show_hour_labels {
        HOUR_LABEL_WIDTH as u16
    } else {
        0
    };
    let selected = &app.selected_event_id;
    let is_focused = app.focus == FocusContext::Timeline;

    if show_hour_labels && is_first {
        let label = Paragraph::new(Line::from(Span::styled(
            " all-day",
            Style::default().fg(app.theme.text_dim),
        )));
        frame.render_widget(label, Rect::new(x, y, label_w, 1));
    }

    // Grid char
    if label_w < width {
        let grid = Paragraph::new(Line::from(Span::styled(
            "│",
            Style::default().fg(app.theme.text_dim),
        )));
        frame.render_widget(grid, Rect::new(x + label_w, y, 1, 1));
    }

    let content_x = x + label_w + 1;
    let content_width = width.saturating_sub(label_w + 1);
    if content_width == 0 {
        return;
    }

    if let Some(event) = event {
        let is_selected = selected.as_deref() == Some(&event.id);
        let is_highlighted = is_selected && is_focused;
        let color = calendar_color(app, event);

        let title = event.display_title();
        let label = format!(" ● {} ", title);
        let truncated = truncate_str(&label, content_width as usize);

        let style = if is_highlighted {
            Style::default()
                .bg(app.theme.selection_bg)
                .fg(app.theme.selection_text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(truncated, style))),
            Rect::new(content_x, y, content_width, 1),
        );
    }
}

fn render_all_day_empty_row(
    app: &AppState,
    frame: &mut Frame,
    x: u16,
    y: u16,
    width: u16,
    show_hour_labels: bool,
    events: &[CalEvent],
    is_collapse_row: bool,
    visible_count: usize,
) {
    let label_w = if show_hour_labels {
        HOUR_LABEL_WIDTH as u16
    } else {
        0
    };

    if label_w < width {
        let grid = Paragraph::new(Line::from(Span::styled(
            "│",
            Style::default().fg(app.theme.text_dim),
        )));
        frame.render_widget(grid, Rect::new(x + label_w, y, 1, 1));
    }

    if is_collapse_row && events.len() > visible_count {
        let hidden = events.len() - visible_count;
        let more_text = format!("  +{} more (a)", hidden);
        let content_x = x + label_w + 1;
        let content_w = width.saturating_sub(label_w + 1);
        if content_w > 0 {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    truncate_str(&more_text, content_w as usize),
                    Style::default().fg(app.theme.text_dim),
                ))),
                Rect::new(content_x, y, content_w, 1),
            );
        }
    }
}

fn render_timeline_grid(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    layout: &DayLayout,
    day: NaiveDate,
    now_slot: i64,
    show_hour_labels: bool,
    available_height: usize,
) {
    let total_slots = 24 * SLOTS_PER_HOUR as usize;
    let start = app.shared_scroll_offset;
    let end = (start + available_height).min(total_slots);

    for (row_idx, slot) in (start..end).enumerate() {
        let y = area.y + row_idx as u16;
        if y >= area.y + area.height {
            break;
        }
        render_slot_row(
            app,
            frame,
            area.x,
            y,
            area.width,
            slot as i64,
            layout,
            now_slot,
            show_hour_labels,
        );
    }
}

fn render_slot_row(
    app: &AppState,
    frame: &mut Frame,
    x: u16,
    y: u16,
    width: u16,
    slot_index: i64,
    layout: &DayLayout,
    now_slot: i64,
    show_hour_labels: bool,
) {
    let label_w = if show_hour_labels {
        HOUR_LABEL_WIDTH as u16
    } else {
        0
    };
    let hour = (slot_index / SLOTS_PER_HOUR) as u32;
    let slot_in_hour = slot_index % SLOTS_PER_HOUR;
    let is_hour_start = slot_in_hour == 0;
    let is_now_slot = slot_index == now_slot;

    // Hour label
    if show_hour_labels && label_w <= width {
        let label_text = if is_hour_start && is_now_slot {
            let s = format_hour_label(hour);
            format!("{:>width$}", s, width = HOUR_LABEL_WIDTH - 1)
        } else if is_hour_start {
            let s = format_hour_label(hour);
            format!("{:>width$}", s, width = HOUR_LABEL_WIDTH - 1)
        } else if is_now_slot {
            format!("{:>width$}", "now", width = HOUR_LABEL_WIDTH - 1)
        } else {
            " ".repeat(HOUR_LABEL_WIDTH - 1)
        };

        let label_style = if is_now_slot {
            Style::default()
                .fg(app.theme.accent_error)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.text_dim)
        };

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("{} ", label_text), // pad to label_w
                label_style,
            ))),
            Rect::new(x, y, label_w, 1),
        );
    }

    // Grid character
    let grid_x = x + label_w;
    if grid_x < x + width {
        let slot_events = layout.get_events_for_slot(slot_index);
        let has_overlap = slot_events.iter().any(|l| l.total_columns > 1);

        let (grid_char, grid_style) = if is_now_slot {
            ("◀", Style::default().fg(app.theme.accent_error))
        } else if is_hour_start {
            let color = if has_overlap {
                app.theme.accent_warning
            } else {
                app.theme.text_dim
            };
            ("┼", Style::default().fg(color))
        } else {
            let color = if has_overlap {
                app.theme.accent_warning
            } else {
                app.theme.text_dim
            };
            ("│", Style::default().fg(color))
        };

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(grid_char, grid_style))),
            Rect::new(grid_x, y, 1, 1),
        );
    }

    // Event content area (after 1-char padding)
    let content_x = x + label_w + 2; // +1 grid, +1 padding
    let content_width = width.saturating_sub(label_w + 2);
    if content_width == 0 {
        return;
    }

    render_slot_events(app, frame, content_x, y, content_width, slot_index, layout);
}

fn render_slot_events(
    app: &AppState,
    frame: &mut Frame,
    x: u16,
    y: u16,
    width: u16,
    slot_index: i64,
    layout: &DayLayout,
) {
    let slot_events = layout.get_events_for_slot(slot_index);
    if slot_events.is_empty() {
        return;
    }

    // Find max columns across all events in this slot
    let max_cols = slot_events
        .iter()
        .map(|l| l.total_columns)
        .max()
        .unwrap_or(1);

    // Build a map column → event_layout
    let mut by_col: std::collections::HashMap<usize, &TimedEventLayout> =
        std::collections::HashMap::new();
    for layout in &slot_events {
        by_col.insert(layout.column, layout);
    }

    let col_width = (width as usize / max_cols).max(1);

    for col in 0..max_cols {
        let col_x = x + (col * col_width) as u16;
        let this_col_width = if col + 1 == max_cols {
            width.saturating_sub((col * col_width) as u16)
        } else {
            col_width as u16
        };
        if this_col_width == 0 {
            continue;
        }

        if let Some(tl) = by_col.get(&col) {
            render_event_cell(app, frame, col_x, y, this_col_width, tl, slot_index);
        }
    }
}

fn render_event_cell(
    app: &AppState,
    frame: &mut Frame,
    x: u16,
    y: u16,
    width: u16,
    tl: &TimedEventLayout,
    slot_index: i64,
) {
    let is_start = minutes_to_slot(tl.start_minutes) == slot_index;
    let event = &tl.event;
    let is_selected = app.selected_event_id.as_deref() == Some(&event.id);
    let is_focused = app.focus == FocusContext::Timeline;
    let is_highlighted = is_selected && is_focused;
    let color = calendar_color(app, event);

    let response_status = event.self_response_status();
    let has_attendees = event.has_other_attendees();
    let is_declined = response_status == Some(&ResponseStatus::Declined);

    // Check if past
    let is_past = get_event_end(event, &app.timezone).map_or(false, |end| {
        end < chrono::Local::now().with_timezone(&end.timezone())
    });
    let should_dim = is_declined || is_past;

    let text_style = if is_highlighted {
        Style::default()
            .bg(app.theme.selection_bg)
            .fg(app.theme.selection_text)
            .add_modifier(Modifier::BOLD)
    } else if should_dim {
        Style::default().fg(app.theme.text_dim)
    } else {
        Style::default().fg(color)
    };

    let content = if is_start {
        let start_dt = get_event_start(event, &app.timezone);
        let time_str = start_dt.map(|dt| format_time(&dt)).unwrap_or_default();
        let title = event.display_title();
        let dot_color = if is_highlighted {
            app.theme.selection_text
        } else {
            color
        };
        let arrow = if is_highlighted { "▸" } else { " " };
        let indicator =
            crate::domain::event::get_attendance_indicator(response_status, has_attendees);
        format!("●{}{} {}{}", arrow, time_str, indicator, title)
    } else {
        "│".to_string()
    };

    let truncated = truncate_str(&content, width as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(truncated, text_style))),
        Rect::new(x, y, width, 1),
    );
}

fn calendar_color(app: &AppState, event: &CalEvent) -> ratatui::style::Color {
    let key = event.calendar_key();
    let base = app.theme.calendar_color(&key);

    // If the calendar key has no real data, fall back to event type color
    if event.account_email.is_none() && event.calendar_id.is_none() {
        return match event.event_type.as_ref() {
            Some(EventType::OutOfOffice) => app.theme.event_out_of_office,
            Some(EventType::FocusTime) => app.theme.event_focus_time,
            Some(EventType::Birthday) => app.theme.event_birthday,
            _ => app.theme.event_default,
        };
    }
    base
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        chars.iter().collect()
    } else {
        let mut result: String = chars[..max_chars.saturating_sub(1)].iter().collect();
        result.push('…');
        result
    }
}
