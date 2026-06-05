use chrono::{Datelike, Duration, NaiveDate};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::domain::time::is_today;
use crate::state::app_state::AppState;

const DOW_LABELS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

pub fn render_month_view(app: &AppState, frame: &mut Frame, area: Rect) {
    if area.width < 7 || area.height < 3 {
        return;
    }

    let anchor = app.view_anchor_day;
    let year = anchor.year();
    let month = anchor.month();

    let first_of_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(anchor);
    // 0=Sun … 6=Sat
    let weekday_offset = first_of_month.weekday().num_days_from_sunday() as i64;
    let grid_start = first_of_month - Duration::days(weekday_offset);

    let last_of_month = last_day_of_month(year, month);
    let total_days_shown = weekday_offset + last_of_month.day() as i64;
    let num_weeks = ((total_days_shown + 6) / 7) as usize;

    // Split: 1 title line + 1 DOW header + num_weeks rows
    let mut row_constraints = vec![
        Constraint::Length(1), // month/year title
        Constraint::Length(1), // DOW header
    ];
    for _ in 0..num_weeks {
        row_constraints.push(Constraint::Fill(1));
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    render_month_title(app, frame, rows[0], year, month);
    render_dow_header(app, frame, rows[1]);

    for week_idx in 0..num_weeks {
        let row_area = rows[week_idx + 2];
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Ratio(1, 7); 7])
            .split(row_area);

        for dow in 0..7usize {
            let day = grid_start + Duration::days((week_idx * 7 + dow) as i64);
            render_day_cell(app, frame, cols[dow], day, month);
        }
    }
}

fn render_month_title(app: &AppState, frame: &mut Frame, area: Rect, year: i32, month: u32) {
    let month_name = month_name(month);
    let title = format!(" {} {}", month_name, year);
    let hint = "H:prev month  L:next month  M:day view";

    let title_width = title.len() as u16;
    let hint_width = hint.len() as u16;
    let padding = area.width.saturating_sub(title_width + hint_width);

    let line = Line::from(vec![
        Span::styled(title, Style::default().add_modifier(Modifier::BOLD).fg(app.theme.accent_primary)),
        Span::raw(" ".repeat(padding as usize)),
        Span::styled(hint, Style::default().fg(app.theme.text_dim)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

fn render_dow_header(app: &AppState, frame: &mut Frame, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Ratio(1, 7); 7])
        .split(area);

    for (i, &label) in DOW_LABELS.iter().enumerate() {
        let is_weekend = i == 0 || i == 6;
        let color = if is_weekend { app.theme.text_weekend } else { app.theme.text_secondary };
        let line = Line::from(vec![
            Span::raw(" "),
            Span::styled(label, Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ]);
        frame.render_widget(Paragraph::new(line), cols[i]);
    }
}

fn render_day_cell(app: &AppState, frame: &mut Frame, area: Rect, day: NaiveDate, current_month: u32) {
    let is_selected = day == app.selected_day;
    let today = is_today(day);
    let in_month = day.month() == current_month;
    let is_weekend = {
        let dow = day.weekday().num_days_from_sunday();
        dow == 0 || dow == 6
    };

    // Choose border style
    let border_style = if is_selected {
        Style::default().fg(app.theme.accent_primary)
    } else if today {
        Style::default().fg(app.theme.accent_success)
    } else {
        Style::default().fg(app.theme.text_dim)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    // Day number line
    let day_num = format!("{:2}", day.day());
    let num_color = if is_selected {
        app.theme.selection_text
    } else if today {
        app.theme.accent_success
    } else if !in_month {
        app.theme.text_dim
    } else if is_weekend {
        app.theme.text_weekend
    } else {
        app.theme.text_secondary
    };

    let day_style = if is_selected {
        Style::default().fg(num_color).bg(app.theme.selection_bg).add_modifier(Modifier::BOLD)
    } else if today {
        Style::default().fg(num_color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(num_color)
    };

    let num_line = Line::from(Span::styled(day_num, day_style));

    // Gather events for this day
    let layout = app.day_layout(day);
    let cell_width = inner.width as usize;
    let available_lines = inner.height.saturating_sub(1) as usize; // -1 for day number

    let mut event_lines: Vec<Line> = Vec::new();
    let all_events = layout.chronological_events();
    for (i, event) in all_events.iter().enumerate() {
        if event_lines.len() >= available_lines {
            let remaining = all_events.len() - i;
            let overflow = format!("+{} more", remaining);
            event_lines.push(Line::from(Span::styled(
                truncate(&overflow, cell_width),
                Style::default().fg(app.theme.text_dim),
            )));
            break;
        }
        let color = app.theme.calendar_color(&event.calendar_key());
        let truncated = truncate(&event.summary, cell_width);
        event_lines.push(Line::from(Span::styled(truncated, Style::default().fg(color))));
    }

    // Build all lines: day number first, then events
    let mut all_lines: Vec<Line> = vec![num_line];
    all_lines.extend(event_lines);

    let para = Paragraph::new(all_lines);
    frame.render_widget(para, inner);
}

fn truncate(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_width {
        s.to_string()
    } else if max_width <= 1 {
        chars[0].to_string()
    } else {
        let truncated: String = chars[..max_width - 1].iter().collect();
        format!("{}…", truncated)
    }
}

fn last_day_of_month(year: i32, month: u32) -> NaiveDate {
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, month, 28).unwrap())
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January", 2 => "February", 3 => "March", 4 => "April",
        5 => "May", 6 => "June", 7 => "July", 8 => "August",
        9 => "September", 10 => "October", 11 => "November", 12 => "December",
        _ => "Unknown",
    }
}

