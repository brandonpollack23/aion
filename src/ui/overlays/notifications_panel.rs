use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::state::app_state::AppState;

pub fn render_notifications_panel(app: &AppState, frame: &mut Frame, area: Rect) {
    let invites = app.pending_invites();

    // Panel anchored to bottom-right, 52 wide × 16 tall
    let width = 52u16.min(area.width);
    let height = 16u16.min(area.height);
    let x = area.x + area.width.saturating_sub(width);
    let y = area.y + area.height.saturating_sub(height + 1);
    let rect = Rect { x, y, width, height };

    frame.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!(
            " Notifications{} ",
            if invites.is_empty() {
                String::new()
            } else {
                format!(" ({})", invites.len())
            }
        ))
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    if invites.is_empty() {
        let msg = Paragraph::new("No pending invites 🎉")
            .style(Style::default().fg(Color::DarkGray));
        let footer = Paragraph::new("esc:close")
            .style(Style::default().fg(Color::DarkGray));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(inner);
        frame.render_widget(msg, chunks[0]);
        frame.render_widget(footer, chunks[1]);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(1)])
        .split(inner);

    let selected = app.notifications_selected_index.min(invites.len().saturating_sub(1));

    let items: Vec<ListItem> = invites
        .iter()
        .enumerate()
        .map(|(i, ev)| {
            let title = ev.display_title();
            let time_str = if let Some(ref dt) = ev.start.date_time {
                chrono::DateTime::parse_from_rfc3339(dt)
                    .map(|d| d.format("%a %b %d %H:%M").to_string())
                    .unwrap_or_else(|_| "??".to_string())
            } else if let Some(ref d) = ev.start.date {
                d.clone()
            } else {
                "?".to_string()
            };
            let organizer = ev
                .organizer
                .as_ref()
                .map(|o| o.display_name.as_deref().unwrap_or(&o.email).to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            if i == selected {
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled("▸ ", Style::default().fg(Color::Yellow)),
                        Span::styled("! ", Style::default().fg(Color::Yellow)),
                        Span::styled(title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::raw("   "),
                        Span::styled(time_str, Style::default().fg(Color::DarkGray)),
                        Span::styled(" · from ", Style::default().fg(Color::DarkGray)),
                        Span::styled(organizer, Style::default().fg(Color::DarkGray)),
                    ]),
                ])
            } else {
                ListItem::new(vec![
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled("! ", Style::default().fg(Color::Yellow)),
                        Span::styled(title, Style::default().fg(Color::Gray)),
                    ]),
                    Line::from(vec![
                        Span::raw("   "),
                        Span::styled(time_str, Style::default().fg(Color::DarkGray)),
                        Span::styled(" · from ", Style::default().fg(Color::DarkGray)),
                        Span::styled(organizer, Style::default().fg(Color::DarkGray)),
                    ]),
                ])
            }
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    let list = List::new(items);
    frame.render_stateful_widget(list, chunks[0], &mut list_state);

    let footer = Paragraph::new(
        Line::from(vec![
            Span::styled("j/k", Style::default().fg(Color::DarkGray)),
            Span::raw(":nav  "),
            Span::styled("y", Style::default().fg(Color::Green)),
            Span::raw(":accept  "),
            Span::styled("n", Style::default().fg(Color::Red)),
            Span::raw(":decline  "),
            Span::styled("m", Style::default().fg(Color::Yellow)),
            Span::raw(":maybe  "),
            Span::styled("esc", Style::default().fg(Color::DarkGray)),
        ]),
    )
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[1]);
}
