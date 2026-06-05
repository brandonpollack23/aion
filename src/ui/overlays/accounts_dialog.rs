use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::auth::tokens::AccountType;
use crate::state::app_state::AppState;

const DIALOG_WIDTH: u16 = 60;
const DIALOG_HEIGHT: u16 = 14;
const SIDEBAR_WIDTH: u16 = 22;

pub fn render_accounts_dialog(app: &AppState, frame: &mut Frame, area: Rect) {
    let popup_rect = centered_rect(DIALOG_WIDTH, DIALOG_HEIGHT, area);
    frame.render_widget(Clear, popup_rect);

    let border_style = Style::default().fg(app.theme.accent_primary);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Span::styled(
            " Accounts ",
            Style::default()
                .fg(app.theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(app.theme.modal_bg));
    let inner = block.inner(popup_rect);
    frame.render_widget(block, popup_rect);

    if app.accounts.is_empty() {
        let msg = Paragraph::new("No accounts connected. Run ':login' to add one.")
            .style(Style::default().fg(app.theme.text_dim));
        frame.render_widget(msg, inner.inner(Margin { horizontal: 1, vertical: 1 }));
        render_footer(app, frame, inner, "Esc:close");
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let body = layout[0];
    let footer_area = layout[1];

    // Split body into sidebar + detail
    let body_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(SIDEBAR_WIDTH),
            Constraint::Fill(1),
        ])
        .split(body);

    render_sidebar(app, frame, body_layout[0]);
    render_detail(app, frame, body_layout[1]);

    let hint = if app.accounts_delete_confirm {
        "y:confirm delete  n/Esc:cancel"
    } else {
        "j/k:navigate  p:set primary  d:remove  Esc:close"
    };
    render_footer(app, frame, inner, hint);
}

fn render_sidebar(app: &AppState, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    for (i, account) in app.accounts.iter().enumerate() {
        let is_selected = i == app.accounts_selected_index;
        let name = account
            .name
            .as_deref()
            .unwrap_or(&account.email)
            .to_string();
        let display = truncate(&name, area.width.saturating_sub(2) as usize);
        let type_badge = match account.account_type {
            AccountType::Google => "G",
            AccountType::CalDAV => "C",
        };
        let label = format!("{} {}", type_badge, display);

        let style = if is_selected {
            Style::default()
                .bg(app.theme.selection_bg)
                .fg(app.theme.selection_text)
        } else {
            Style::default().fg(app.theme.text_primary)
        };
        lines.push(Line::from(Span::styled(label, style)));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn render_detail(app: &AppState, frame: &mut Frame, area: Rect) {
    let inner = area.inner(Margin { horizontal: 1, vertical: 0 });
    let Some(account) = app.accounts.get(app.accounts_selected_index) else {
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    let type_str = match account.account_type {
        AccountType::Google => "Google",
        AccountType::CalDAV => "CalDAV",
    };
    let type_color = match account.account_type {
        AccountType::Google => app.theme.accent_success,
        AccountType::CalDAV => app.theme.accent_warning,
    };

    lines.push(Line::from(vec![
        Span::styled("type  ", Style::default().fg(app.theme.text_dim)),
        Span::styled(type_str, Style::default().fg(type_color)),
    ]));

    let email_display = truncate(&account.email, inner.width.saturating_sub(6) as usize);
    lines.push(Line::from(vec![
        Span::styled("email ", Style::default().fg(app.theme.text_dim)),
        Span::styled(email_display, Style::default().fg(app.theme.text_primary)),
    ]));

    let name_display = account
        .name
        .as_deref()
        .map(|n| truncate(n, inner.width.saturating_sub(6) as usize))
        .unwrap_or_default();
    if !name_display.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("name  ", Style::default().fg(app.theme.text_dim)),
            Span::styled(name_display, Style::default().fg(app.theme.text_primary)),
        ]));
    }

    let store = crate::auth::tokens::load_accounts_store();
    let is_primary = store.default_account.as_deref() == Some(&account.email);
    if is_primary {
        lines.push(Line::from(Span::styled(
            "  Primary account",
            Style::default().fg(app.theme.accent_success),
        )));
    }

    if app.accounts_delete_confirm {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "Remove this account? (y/n)",
            Style::default().fg(app.theme.accent_error),
        )));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn render_footer(app: &AppState, frame: &mut Frame, area: Rect, hint: &str) {
    let footer_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    let para = Paragraph::new(Span::styled(hint, Style::default().fg(app.theme.text_dim)));
    frame.render_widget(para, footer_area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", cut)
    }
}
