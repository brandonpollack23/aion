use chrono::Timelike;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::state::app_state::AppState;
use crate::state::meet_with::MeetWithStep;

const DIALOG_WIDTH: u16 = 64;
const DIALOG_HEIGHT: u16 = 20;

pub fn render_meet_with_dialog(app: &AppState, frame: &mut Frame, area: Rect) {
    let mw = match app.meet_with.as_ref() {
        Some(s) => s,
        None => return,
    };

    let width = DIALOG_WIDTH.min(area.width);
    let height = DIALOG_HEIGHT.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let rect = Rect { x, y, width, height };

    frame.render_widget(Clear, rect);

    let step_label = if mw.step == MeetWithStep::People {
        "Meet With (1/2)"
    } else {
        "Available Slots (2/2)"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" {} ", step_label))
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    if mw.step == MeetWithStep::People {
        render_people_step(app, frame, inner);
    } else {
        render_slots_step(app, frame, inner);
    }
}

fn render_people_step(app: &AppState, frame: &mut Frame, area: Rect) {
    let mw = app.meet_with.as_ref().unwrap();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // search label + query
            Constraint::Length(1), // selected chips
            Constraint::Fill(1),   // contact list
            Constraint::Length(1), // footer
        ])
        .split(area);

    // Search field
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("search: ", Style::default().fg(Color::Cyan)),
            Span::styled(&mw.search_query, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Cyan)),
        ])),
        rows[0],
    );

    // Selected people chips
    let chips = if mw.selected_people.is_empty() {
        Paragraph::new(Span::styled(
            "No people selected yet",
            Style::default().fg(Color::DarkGray),
        ))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled("with: ", Style::default().fg(Color::DarkGray)),
            Span::styled(mw.selected_names_display(), Style::default().fg(Color::Green)),
        ]))
    };
    frame.render_widget(chips, rows[1]);

    // Contact list
    let contacts = mw.filtered_contacts();
    let cursor = mw.list_cursor.min(contacts.len().saturating_sub(1));

    if contacts.is_empty() {
        let hint = if mw.search_query.contains('@') {
            "Press Enter to add this email"
        } else {
            "No contacts. Type an email address."
        };
        frame.render_widget(
            Paragraph::new(Span::styled(hint, Style::default().fg(Color::DarkGray))),
            rows[2],
        );
    } else {
        let max_visible = rows[2].height as usize;
        let start = cursor.saturating_sub(max_visible / 2);
        let end = (start + max_visible).min(contacts.len());
        let visible = &contacts[start..end];

        let items: Vec<ListItem> = visible
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let real_idx = start + i;
                let selected = mw.is_selected(&c.email);
                let highlight = real_idx == cursor;
                let checkbox = if selected { "☑" } else { "☐" };
                let name = c.display_name.as_deref().unwrap_or(&c.email);

                let style = if highlight {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else if selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Gray)
                };
                ListItem::new(format!(" {} {}", checkbox, name)).style(style)
            })
            .collect();

        frame.render_widget(List::new(items), rows[2]);
    }

    // Footer
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::DarkGray)),
            Span::raw(":nav  "),
            Span::styled("Enter/Tab", Style::default().fg(Color::DarkGray)),
            Span::raw(":select  "),
            Span::styled("^→", Style::default().fg(Color::DarkGray)),
            Span::raw(":next  "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(":cancel"),
        ])),
        rows[3],
    );
}

fn render_slots_step(app: &AppState, frame: &mut Frame, area: Rect) {
    let mw = app.meet_with.as_ref().unwrap();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // "with: ..." + duration
            Constraint::Fill(1),   // slots list + visualizer
            Constraint::Length(1), // footer
        ])
        .split(area);

    // Header: participants + duration
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("with: ", Style::default().fg(Color::DarkGray)),
            Span::styled(mw.selected_names_display(), Style::default().fg(Color::White)),
            Span::raw("  "),
            Span::styled("dur: ", Style::default().fg(Color::DarkGray)),
            Span::styled(mw.duration_display(), Style::default().fg(Color::Cyan)),
            Span::styled(" (←→)", Style::default().fg(Color::DarkGray)),
        ])),
        rows[0],
    );

    // Slots area
    if mw.slots_loading {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "Checking availability…",
                Style::default().fg(Color::Yellow),
            )),
            rows[1],
        );
    } else if mw.slots.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "No available slots found in the next 7 days.",
                Style::default().fg(Color::DarkGray),
            )),
            rows[1],
        );
    } else {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Fill(1)])
            .split(rows[1]);

        render_slots_list(app, frame, cols[0]);
        render_slot_visualizer(app, frame, cols[1]);
    }

    // Footer
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("j/k", Style::default().fg(Color::DarkGray)),
            Span::raw(":nav  "),
            Span::styled("←/→", Style::default().fg(Color::DarkGray)),
            Span::raw(":duration  "),
            Span::styled("Enter", Style::default().fg(Color::DarkGray)),
            Span::raw(":create  "),
            Span::styled("b", Style::default().fg(Color::DarkGray)),
            Span::raw(":back  "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
        ])),
        rows[2],
    );
}

fn render_slots_list(app: &AppState, frame: &mut Frame, area: Rect) {
    let mw = app.meet_with.as_ref().unwrap();
    let slots = &mw.slots;
    let cursor = mw.slots_cursor.min(slots.len().saturating_sub(1));
    let max_visible = area.height as usize;

    // Find items (slots with busy-gap separators)
    let start = cursor.saturating_sub(max_visible / 2);
    let end = (start + max_visible).min(slots.len());
    let visible = &slots[start..end];

    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let real_idx = start + i;
            let hl = real_idx == cursor;

            let slot_str = format!(
                "{} {}–{}",
                slot.start.format("%a %b %d"),
                slot.start.format("%H:%M"),
                slot.end.format("%H:%M"),
            );

            let style = if hl {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::Gray)
            };
            let prefix = if hl { "▸ " } else { "  " };
            ListItem::new(format!("{}{}", prefix, slot_str)).style(style)
        })
        .collect();

    frame.render_widget(List::new(items), area);
}

fn render_slot_visualizer(app: &AppState, frame: &mut Frame, area: Rect) {
    let mw = app.meet_with.as_ref().unwrap();
    let slots = &mw.slots;
    if slots.is_empty() {
        return;
    }
    let cursor = mw.slots_cursor.min(slots.len().saturating_sub(1));
    let slot = &slots[cursor];

    let bar_width = (area.width as usize).min(20);

    // Determine display range: 1h before earliest slot on this day, 1h after latest
    let day_str = slot.start.format("%Y-%m-%d").to_string();
    let day_hours: Vec<f32> = slots
        .iter()
        .filter(|s| s.start.format("%Y-%m-%d").to_string() == day_str)
        .flat_map(|s| {
            let sh = s.start.hour() as f32 + s.start.minute() as f32 / 60.0;
            let eh = s.end.hour() as f32 + s.end.minute() as f32 / 60.0;
            vec![sh, eh]
        })
        .collect();

    let min_hour = day_hours.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_hour = day_hours.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let range_start = (min_hour - 1.0).max(0.0).floor() as u32;
    let range_end = (max_hour + 1.0).min(24.0).ceil() as u32;
    let total_hours = (range_end - range_start).max(4) as f32;

    // Build timeline bar
    let slot_start_h = slot.start.hour() as f32 + slot.start.minute() as f32 / 60.0;
    let slot_end_h = slot.end.hour() as f32 + slot.end.minute() as f32 / 60.0;

    let bar: String = (0..bar_width)
        .map(|i| {
            let char_start_h = range_start as f32 + (i as f32 / bar_width as f32) * total_hours;
            let char_end_h = range_start as f32 + ((i + 1) as f32 / bar_width as f32) * total_hours;
            if char_start_h < slot_end_h && char_end_h > slot_start_h {
                '#'
            } else {
                '-'
            }
        })
        .collect();

    let mid_hour = (range_start + range_end) / 2;

    let label_line = format!(
        "{:<4}{:>6}{:>8}",
        format!("{}a", range_start),
        if mid_hour < 12 { format!("{}a", mid_hour) } else { format!("{}p", mid_hour - 12) },
        if range_end <= 12 { format!("{}a", range_end) } else { format!("{}p", range_end - 12) },
    );

    let lines = vec![
        Line::from(Span::styled(
            slot.start.format("%A").to_string(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            slot.start.format("%B %d, %Y").to_string(),
            Style::default().fg(Color::Gray),
        )),
        Line::from(vec![
            Span::styled("│", Style::default().fg(Color::DarkGray)),
            Span::styled(
                bar,
                Style::default().fg(Color::Cyan),
            ),
            Span::styled("│", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(Span::styled(
            label_line,
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::raw("")),
        Line::from(Span::styled(
            format!(
                "{} – {}",
                slot.start.format("%H:%M"),
                slot.end.format("%H:%M")
            ),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("({} min)", slot.duration_minutes),
            Style::default().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}
