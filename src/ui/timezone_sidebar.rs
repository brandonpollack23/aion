use chrono::{Local, Offset, Timelike};
use chrono_tz::Tz;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::domain::layout::SLOTS_PER_HOUR;
use crate::domain::time::{format_hour_label, get_now_minutes, parse_tz};
use crate::state::app_state::AppState;

pub const TZ_COL_WIDTH: usize = 7;

/// Returns the ordered list of timezone strings to display: [additional..., local].
/// additional_timezones is capped at 4; local is always appended last (rightmost).
pub fn tz_display_list(local_tz: &str, additional: &[String]) -> Vec<String> {
    let mut list: Vec<String> = additional.iter().take(4).cloned().collect();
    list.push(local_tz.to_string());
    list
}

fn tz_abbrev(tz_str: &str) -> String {
    // Use the last component of the IANA name as the abbreviation (e.g. "Los_Angeles" → "LA", "UTC" → "UTC")
    // For display, just take the last segment and truncate to 6 chars
    let abbrev = tz_str.rsplit('/').next().unwrap_or(tz_str);
    let abbrev = abbrev.replace('_', " ");
    if abbrev.chars().count() > 6 {
        abbrev.chars().take(5).collect::<String>() + "…"
    } else {
        abbrev.to_string()
    }
}

fn tz_utc_offset_string(tz: Tz) -> String {
    use chrono::TimeZone;
    let now = Local::now().naive_utc();
    let offset_secs = tz.from_utc_datetime(&now).offset().fix().local_minus_utc();
    let h = offset_secs / 3600;
    let m = (offset_secs.abs() % 3600) / 60;
    if m == 0 {
        format!("UTC{:+}", h)
    } else {
        format!("UTC{:+}:{:02}", h, m)
    }
}

pub fn render_timezone_sidebar(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    all_day_lines: usize,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let tz_list = tz_display_list(&app.timezone, &app.config.view.additional_timezones);
    let num_tz = tz_list.len();
    let col_w = TZ_COL_WIDTH as u16;

    // header occupies row 0: day header line (blank in sidebar, but must match timeline column)
    // all_day_lines rows after that
    // then timeline slots
    let header_y = area.y;
    let all_day_y = area.y + 1;
    let timeline_y = area.y + 1 + all_day_lines as u16;
    let timeline_height = area.height.saturating_sub(1 + all_day_lines as u16) as usize;

    let total_slots = 24 * SLOTS_PER_HOUR as usize;
    let start = app.shared_scroll_offset;
    let end = (start + timeline_height).min(total_slots);

    // Render each timezone column
    for (col_idx, tz_str) in tz_list.iter().enumerate() {
        let is_local = col_idx == num_tz - 1;
        let col_x = area.x + (col_idx as u16) * col_w;
        if col_x + col_w > area.x + area.width {
            break;
        }

        let tz_opt = parse_tz(tz_str);

        // --- Header row (row 0): timezone abbreviated name ---
        {
            let abbrev = tz_abbrev(tz_str);
            let padded = format!("{:>width$} ", abbrev, width = TZ_COL_WIDTH - 1);
            let style = if is_local {
                Style::default()
                    .fg(app.theme.accent_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.text_dim)
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(padded, style))),
                Rect::new(col_x, header_y, col_w, 1),
            );
        }

        // --- All-day rows: show UTC offset on first row, blank otherwise ---
        if all_day_lines > 0 {
            let offset_str = tz_opt
                .map(tz_utc_offset_string)
                .unwrap_or_else(|| "???".to_string());
            let padded = format!("{:>width$} ", offset_str, width = TZ_COL_WIDTH - 1);
            let style = if is_local {
                Style::default().fg(app.theme.accent_primary)
            } else {
                Style::default().fg(app.theme.text_dim)
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(padded, style))),
                Rect::new(col_x, all_day_y, col_w, 1),
            );
            // remaining all-day rows: blank
            for row in 1..all_day_lines {
                let y = all_day_y + row as u16;
                if is_local {
                    frame.render_widget(
                        Paragraph::new(Line::from(Span::styled(
                            " ".repeat(TZ_COL_WIDTH),
                            Style::default().bg(app.theme.timezone_local_bg),
                        ))),
                        Rect::new(col_x, y, col_w, 1),
                    );
                }
            }
        }

        // --- Timeline slot rows ---
        let now_minutes = get_now_minutes();
        let now_slot = crate::domain::layout::minutes_to_slot(now_minutes);

        for (row_idx, slot) in (start..end).enumerate() {
            let y = timeline_y + row_idx as u16;
            if y >= area.y + area.height {
                break;
            }

            let slot_i = slot as i64;
            let hour = (slot_i / SLOTS_PER_HOUR) as u32;
            let slot_in_hour = slot_i % SLOTS_PER_HOUR;
            let is_hour_start = slot_in_hour == 0;
            let is_now = slot_i == now_slot;

            let label_text = if is_now && !is_hour_start {
                // show "now" on non-local columns too, but only if it's the now slot
                format!("{:>width$} ", "now", width = TZ_COL_WIDTH - 1)
            } else if is_hour_start {
                // Convert hour to the target timezone
                let display_hour = if let Some(tz) = tz_opt {
                    use chrono::TimeZone;
                    let local_now = Local::now();
                    // Get current date in local time, then build a datetime at `hour:00` in local tz
                    let naive = local_now.date_naive().and_hms_opt(hour, 0, 0).unwrap();
                    let local_dt = chrono::Local.from_local_datetime(&naive).single();
                    local_dt
                        .map(|dt| {
                            let in_tz = dt.with_timezone(&tz);
                            in_tz.hour()
                        })
                        .unwrap_or(hour)
                } else {
                    hour
                };
                let s = format_hour_label(display_hour);
                format!("{:>width$} ", s, width = TZ_COL_WIDTH - 1)
            } else {
                " ".repeat(TZ_COL_WIDTH)
            };

            let style = if is_now {
                Style::default()
                    .fg(app.theme.accent_error)
                    .add_modifier(Modifier::BOLD)
            } else if is_local {
                Style::default().fg(app.theme.accent_primary)
            } else {
                Style::default().fg(app.theme.text_dim)
            };

            let final_style = if is_local && !is_now {
                style.bg(app.theme.timezone_local_bg)
            } else {
                style
            };

            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(label_text, final_style))),
                Rect::new(col_x, y, col_w, 1),
            );
        }
    }
}
