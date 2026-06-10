use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::state::app_state::AppState;
use crate::ui::calendars_sidebar::{render_calendars_sidebar, CALENDARS_SIDEBAR_WIDTH};
use crate::ui::days_sidebar::{render_days_sidebar, DAYS_SIDEBAR_WIDTH};
use crate::ui::timeline_column::render_timeline_column;
use crate::ui::timezone_sidebar::{render_timezone_sidebar, TZ_COL_WIDTH};

pub fn render_day_view(app: &AppState, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let days_in_view = app.days_in_view();

    // Compute max all-day events across all visible columns
    let max_all_day: usize = days_in_view
        .iter()
        .map(|&day| app.day_layout(day).all_day_events.len())
        .max()
        .unwrap_or(0);

    // All-day lines: visible + 1 collapse row if needed, or 0 if none
    let all_day_lines: usize = if max_all_day == 0 {
        0
    } else if max_all_day <= 2 || app.all_day_expanded {
        max_all_day
            + if app.all_day_expanded && max_all_day > 2 {
                1
            } else {
                0
            }
    } else {
        3 // 2 visible + 1 "+N more" row
    };

    // Build constraints for horizontal split
    let num_tz = (app.config.view.additional_timezones.len().min(4) + 1) as u16;
    let tz_sidebar_width = num_tz * TZ_COL_WIDTH as u16;

    let mut constraints = Vec::new();
    if app.calendar_sidebar_visible {
        constraints.push(Constraint::Length(CALENDARS_SIDEBAR_WIDTH));
    }
    constraints.push(Constraint::Length(DAYS_SIDEBAR_WIDTH));
    constraints.push(Constraint::Length(tz_sidebar_width));
    for _ in 0..app.column_count {
        constraints.push(Constraint::Fill(1));
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;

    // Calendars sidebar
    if app.calendar_sidebar_visible {
        render_calendars_sidebar(app, frame, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Days sidebar
    render_days_sidebar(app, frame, chunks[chunk_idx]);
    chunk_idx += 1;

    // Timezone sidebar (replaces the inline hour labels)
    render_timezone_sidebar(app, frame, chunks[chunk_idx], all_day_lines);
    chunk_idx += 1;

    // Timeline columns (hour labels disabled — timezone sidebar handles time display)
    for (col_idx, &day) in days_in_view.iter().enumerate() {
        if chunk_idx >= chunks.len() {
            break;
        }
        render_timeline_column(
            app,
            frame,
            chunks[chunk_idx],
            day,
            col_idx,
            false, // show_hour_labels: timezone sidebar owns time display
            max_all_day,
            all_day_lines,
        );
        chunk_idx += 1;
    }
}
