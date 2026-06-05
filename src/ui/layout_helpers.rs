use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Returns a centered rectangle of fixed pixel dimensions.
pub fn centered_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}

/// Returns a centered rectangle using percentage of parent.
pub fn centered_percent(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let h_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(h_split[1])[1]
}

/// Returns a right-aligned panel of fixed width occupying the full height.
pub fn right_panel(width: u16, r: Rect) -> Rect {
    Rect {
        x: r.x + r.width.saturating_sub(width),
        y: r.y,
        width: width.min(r.width),
        height: r.height,
    }
}

/// Shrink a rect by uniform margin.
pub fn inner_margin(r: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: r.x + horizontal,
        y: r.y + vertical,
        width: r.width.saturating_sub(horizontal * 2),
        height: r.height.saturating_sub(vertical * 2),
    }
}
