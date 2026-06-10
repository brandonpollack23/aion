use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;

use crate::app::AppEvent;
use crate::config::schema::ViewMode;
use crate::domain::natural_date::parse_natural_date;
use crate::state::app_state::{AppState, SelectionDirection};
use crate::state::dialog::DialogState;
use crate::state::focus::FocusContext;
use crate::state::overlay::{Overlay, OverlayKind, RecurrenceScope};

const DOUBLE_KEY_TIMEOUT: Duration = Duration::from_millis(500);

/// Returns true if the app should quit.
pub async fn handle_key(app: &mut AppState, key: KeyEvent, tx: &UnboundedSender<AppEvent>) -> bool {
    // Ctrl+C always quits
    if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
        return true;
    }

    match &app.focus.clone() {
        FocusContext::Timeline | FocusContext::Days | FocusContext::Calendars => {
            handle_navigation(app, key, tx).await
        }
        FocusContext::Command => {
            handle_command_input(app, key, tx).await;
            false
        }
        FocusContext::Search => {
            handle_search_input(app, key, tx).await;
            false
        }
        FocusContext::Details => {
            handle_details_input(app, key, tx);
            false
        }
        FocusContext::Confirm => {
            handle_confirm_input(app, key, tx);
            false
        }
        FocusContext::Dialog => {
            handle_dialog_input(app, key, tx);
            false
        }
        FocusContext::Goto => {
            handle_goto_input(app, key);
            false
        }
        FocusContext::Messages => {
            handle_messages_input(app, key);
            false
        }
        FocusContext::Accounts => {
            handle_accounts_input(app, key, tx);
            false
        }
        FocusContext::CalDavLogin => {
            handle_caldav_login_input(app, key, tx).await;
            false
        }
        FocusContext::Notifications => {
            handle_notifications_input(app, key, tx).await;
            false
        }
        FocusContext::ProposeTime => {
            handle_propose_time_input(app, key, tx).await;
            false
        }
        FocusContext::MeetWith => {
            handle_meet_with_input(app, key, tx).await;
            false
        }
        FocusContext::LoginPrompt => {
            handle_login_prompt_input(app, key, tx);
            false
        }
        _ => {
            // Generic overlay: Escape pops
            if key.code == KeyCode::Esc {
                app.pop_overlay();
            }
            false
        }
    }
}

async fn handle_navigation(
    app: &mut AppState,
    key: KeyEvent,
    tx: &UnboundedSender<AppEvent>,
) -> bool {
    let mods = key.modifiers;
    let no_mods = mods == KeyModifiers::NONE;
    let shift = mods == KeyModifiers::SHIFT;
    let ctrl = mods == KeyModifiers::CONTROL;

    // --- Double-g detection ---
    if key.code == KeyCode::Char('g') && no_mods {
        let now = Instant::now();
        if let Some((KeyCode::Char('g'), prev_time)) = app.last_key {
            if now.duration_since(prev_time) < DOUBLE_KEY_TIMEOUT {
                app.last_key = None;
                match app.focus {
                    FocusContext::Timeline => app.move_event_selection(SelectionDirection::First),
                    FocusContext::Days => {
                        let days = app.days_list(20);
                        if let Some(&first) = days.first() {
                            app.selected_day = first;
                            app.view_anchor_day = first;
                        }
                    }
                    FocusContext::Calendars => {
                        app.selected_calendar_index = 0;
                        app.clamp_calendar_scroll();
                    }
                    _ => {}
                }
                return false;
            }
        }
        app.last_key = Some((KeyCode::Char('g'), now));
        return false;
    }

    // Clear double-key if different key
    if key.code != KeyCode::Char('g') {
        app.last_key = None;
    }

    match key.code {
        // Quit
        KeyCode::Char('q') if no_mods => return true,

        // Toggle focus between Timeline / Days / Calendars
        KeyCode::Char('`') if no_mods => cycle_focus(app),

        // j / ↓
        KeyCode::Char('j') | KeyCode::Down if no_mods => {
            if app.view_mode == ViewMode::Monthly {
                app.move_day_selection(7);
            } else {
                match app.focus {
                    FocusContext::Timeline => app.move_event_selection(SelectionDirection::Next),
                    FocusContext::Days => {
                        app.move_day_selection(1);
                        app.focus = FocusContext::Days;
                    }
                    FocusContext::Calendars => {
                        if app.selected_calendar_index + 1 < app.calendars.len() {
                            app.selected_calendar_index += 1;
                        }
                        app.clamp_calendar_scroll();
                    }
                    _ => {}
                }
            }
        }

        // k / ↑
        KeyCode::Char('k') | KeyCode::Up if no_mods => {
            if app.view_mode == ViewMode::Monthly {
                app.move_day_selection(-7);
            } else {
                match app.focus {
                    FocusContext::Timeline => app.move_event_selection(SelectionDirection::Prev),
                    FocusContext::Days => {
                        app.move_day_selection(-1);
                        app.focus = FocusContext::Days;
                    }
                    FocusContext::Calendars => {
                        app.selected_calendar_index = app.selected_calendar_index.saturating_sub(1);
                        app.clamp_calendar_scroll();
                    }
                    _ => {}
                }
            }
        }

        // h / ←  and  l / →
        KeyCode::Char('h') | KeyCode::Left if no_mods => {
            if app.view_mode == ViewMode::Monthly {
                app.move_day_selection(-1);
            } else {
                app.move_column(-1);
            }
        }
        KeyCode::Char('l') | KeyCode::Right if no_mods => {
            if app.view_mode == ViewMode::Monthly {
                app.move_day_selection(1);
            } else {
                app.move_column(1);
            }
        }

        // H / L: scroll months (prev/next)
        KeyCode::Char('H') if shift => app.move_month(-1),
        KeyCode::Char('L') if shift => app.move_month(1),

        // G: last event / last day
        KeyCode::Char('G') | KeyCode::Char('g') if shift => match app.focus {
            FocusContext::Timeline => app.move_event_selection(SelectionDirection::Last),
            FocusContext::Days => {
                let days = app.days_list(20);
                if let Some(&last) = days.last() {
                    app.selected_day = last;
                    app.view_anchor_day = last;
                }
            }
            FocusContext::Calendars => {
                if !app.calendars.is_empty() {
                    app.selected_calendar_index = app.calendars.len() - 1;
                }
                app.clamp_calendar_scroll();
            }
            _ => {}
        },

        // n: jump to now
        KeyCode::Char('n') if no_mods => app.jump_to_now(),

        // Enter / Space
        KeyCode::Enter | KeyCode::Char(' ') if no_mods => {
            if app.view_mode == ViewMode::Monthly {
                app.toggle_view_mode();
            } else {
                match app.focus {
                    FocusContext::Days => {
                        app.focus = FocusContext::Timeline;
                        app.auto_select_event();
                    }
                    FocusContext::Calendars => toggle_calendar(app),
                    FocusContext::Timeline => open_details(app),
                    _ => {}
                }
            }
        }

        // e: open edit dialog
        KeyCode::Char('e') if no_mods => {
            if app.focus == FocusContext::Timeline {
                open_event_dialog_edit(app);
            }
        }

        // N (shift): new event
        KeyCode::Char('N') if shift => open_event_dialog_new(app),

        // D: delete selected event
        KeyCode::Char('D') if shift => {
            if app.focus == FocusContext::Timeline {
                initiate_delete(app);
            }
        }

        // a: toggle all-day expanded
        KeyCode::Char('a') if no_mods => app.all_day_expanded = !app.all_day_expanded,

        // d: set selected calendar as the new-event destination
        KeyCode::Char('d') if no_mods && app.focus == FocusContext::Calendars => {
            select_new_event_calendar(app);
        }

        // 3: cycle column view (1→3→5→1)
        KeyCode::Char('3') if no_mods => app.toggle_column_count(),

        // Z: toggle timezone display in header
        KeyCode::Char('Z') if shift => {
            app.show_timezone = !app.show_timezone;
        }

        // C (shift): toggle calendar sidebar
        KeyCode::Char('C') if shift => {
            if app.calendar_sidebar_visible {
                app.calendar_sidebar_visible = false;
                if app.focus == FocusContext::Calendars {
                    app.focus = FocusContext::Timeline;
                }
            } else {
                app.calendar_sidebar_visible = true;
                app.focus = FocusContext::Calendars;
            }
        }

        // /: start search
        KeyCode::Char('/') if no_mods => {
            app.focus = FocusContext::Search;
            app.search_query = String::new();
            app.search_results = Vec::new();
        }

        // :: open command
        KeyCode::Char(':') if no_mods => {
            app.focus = FocusContext::Command;
            app.command_input = String::new();
            app.command_cursor = 0;
            app.command_selected_index = 0;
        }

        // ?: open help overlay
        KeyCode::Char('?') if no_mods => {
            let prev = app.focus.clone();
            app.push_overlay(Overlay {
                kind: OverlayKind::Help,
                prev_focus: Some(prev),
            });
        }

        // A: open accounts dialog
        KeyCode::Char('A') if shift => {
            let prev = app.focus.clone();
            app.push_overlay(Overlay {
                kind: OverlayKind::Accounts,
                prev_focus: Some(prev),
            });
        }

        // I (shift): open notifications (invites)
        KeyCode::Char('I') if shift => {
            let prev = app.focus.clone();
            app.notifications_selected_index = 0;
            app.push_overlay(Overlay {
                kind: OverlayKind::Notifications,
                prev_focus: Some(prev),
            });
        }

        // M (shift): toggle month/day view
        KeyCode::Char('M') if shift => app.toggle_view_mode(),

        // Ctrl+M: open MeetWith dialog
        KeyCode::Char('m') if ctrl => {
            let contacts = crate::api::contacts::merged_contacts(&app.events);
            let prev = app.focus.clone();
            app.meet_with = Some(crate::state::meet_with::MeetWithState::new(contacts));
            app.push_overlay(Overlay {
                kind: OverlayKind::MeetWith,
                prev_focus: Some(prev),
            });
        }

        _ => {}
    }
    false
}

fn open_details(app: &mut AppState) {
    if app.selected_event_id.is_none() {
        return;
    }
    let prev = app.focus.clone();
    app.push_overlay(Overlay {
        kind: OverlayKind::Details,
        prev_focus: Some(prev),
    });
}

fn initiate_delete(app: &mut AppState) {
    let id = match app.selected_event_id.clone() {
        Some(id) => id,
        None => return,
    };
    let is_recurring = app.events.get(&id).map_or(false, |e| e.is_recurring());

    if is_recurring {
        app.pending_confirm_is_delete = true;
        app.recurrence_scope = RecurrenceScope::This;
        let prev = app.focus.clone();
        app.push_overlay(Overlay {
            kind: OverlayKind::Confirm,
            prev_focus: Some(prev),
        });
    } else {
        // Non-recurring: delete directly (Phase 3 will call API)
        app.events.remove(&id);
        app.selected_event_id = None;
        app.auto_select_event();
    }
}

fn handle_details_input(app: &mut AppState, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
    let no_mods = key.modifiers == KeyModifiers::NONE;
    let shift = key.modifiers == KeyModifiers::SHIFT;

    match key.code {
        KeyCode::Esc => app.pop_overlay(),

        // e: edit
        KeyCode::Char('e') if no_mods => {
            app.pop_overlay();
            open_event_dialog_edit(app);
        }

        // D: delete
        KeyCode::Char('D') if shift => {
            app.pop_overlay();
            initiate_delete(app);
        }

        // RSVP: y / n / m
        KeyCode::Char('y') if no_mods => rsvp_from_details(app, "accepted", tx),
        KeyCode::Char('n') if no_mods => rsvp_from_details(app, "declined", tx),
        KeyCode::Char('m') if no_mods => rsvp_from_details(app, "tentative", tx),

        // p: propose new time
        KeyCode::Char('p') if no_mods => {
            if let Some(ref id) = app.selected_event_id.clone() {
                if let Some(event) = app.events.get(id).cloned() {
                    app.pop_overlay();
                    open_propose_time_dialog(app, &event);
                }
            }
        }

        // o: open meeting link
        KeyCode::Char('o') if no_mods => {
            if let Some(ref id) = app.selected_event_id.clone() {
                if let Some(event) = app.events.get(id) {
                    if let Some(ref link) = event.hangout_link.clone() {
                        let _ = open::that(link);
                    }
                }
            }
        }

        _ => {}
    }
}

fn handle_confirm_input(app: &mut AppState, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
    let no_mods = key.modifiers == KeyModifiers::NONE;
    let is_rsvp = app.pending_rsvp_status.is_some();

    match key.code {
        KeyCode::Esc => {
            app.pending_rsvp_status = None;
            app.pop_overlay();
        }

        // j / ↓: cycle scope forward
        KeyCode::Char('j') | KeyCode::Down if no_mods => {
            app.recurrence_scope = match app.recurrence_scope {
                RecurrenceScope::This => RecurrenceScope::Following,
                RecurrenceScope::Following if is_rsvp => RecurrenceScope::Following,
                RecurrenceScope::Following => RecurrenceScope::All,
                RecurrenceScope::All => RecurrenceScope::All,
            };
        }

        // k / ↑: cycle scope backward
        KeyCode::Char('k') | KeyCode::Up if no_mods => {
            app.recurrence_scope = match app.recurrence_scope {
                RecurrenceScope::This => RecurrenceScope::This,
                RecurrenceScope::Following => RecurrenceScope::This,
                RecurrenceScope::All => RecurrenceScope::Following,
            };
        }

        // Enter: confirm action
        KeyCode::Enter => {
            let id = app.selected_event_id.clone();
            let is_delete = app.pending_confirm_is_delete;
            let scope = app.recurrence_scope.clone();
            let rsvp_status = app.pending_rsvp_status.take();
            app.pop_overlay();
            if let Some(id) = id {
                if let Some(status) = rsvp_status {
                    apply_rsvp(app, &id, &scope, &status, tx);
                } else if is_delete {
                    apply_delete(app, &id, &scope);
                }
                // Phase 3: edit with scope handled similarly
            }
        }

        _ => {}
    }
}

fn apply_delete(app: &mut AppState, id: &str, scope: &RecurrenceScope) {
    let recurring_id = app
        .events
        .get(id)
        .and_then(|e| e.recurring_event_id.clone());

    match scope {
        RecurrenceScope::This => {
            app.events.remove(id);
        }
        RecurrenceScope::Following => {
            // Remove this and all future instances sharing the same recurring_event_id
            // Simple heuristic: remove events with same recurring_event_id and start >= this one
            let base_start = app
                .events
                .get(id)
                .and_then(|e| e.start.date_time.clone().or(e.start.date.clone()));
            if let (Some(ref rid), Some(ref cutoff)) = (recurring_id, base_start) {
                let rid = rid.clone();
                let cutoff = cutoff.clone();
                app.events.retain(|_, e| {
                    if e.recurring_event_id.as_deref() == Some(&rid) {
                        let e_start = e
                            .start
                            .date_time
                            .as_deref()
                            .or(e.start.date.as_deref())
                            .unwrap_or("");
                        e_start < cutoff.as_str()
                    } else {
                        true
                    }
                });
            } else {
                app.events.remove(id);
            }
        }
        RecurrenceScope::All => {
            if let Some(rid) = recurring_id {
                app.events
                    .retain(|_, e| e.recurring_event_id.as_deref() != Some(&rid) && e.id != id);
            } else {
                app.events.remove(id);
            }
        }
    }
    app.selected_event_id = None;
    app.auto_select_event();
}

async fn handle_command_input(app: &mut AppState, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    match key.code {
        KeyCode::Esc => {
            app.focus = FocusContext::Timeline;
            app.command_input.clear();
            app.command_cursor = 0;
            app.command_completion_query = None;
        }
        KeyCode::Enter => {
            let cmd = app.command_input.trim().to_string();
            execute_command(app, &cmd, tx);
            app.focus = FocusContext::Timeline;
            app.command_input.clear();
            app.command_cursor = 0;
            app.command_completion_query = None;
        }
        KeyCode::Backspace if ctrl => {
            cmd_kill_word_backward(app);
            app.command_completion_query = None;
        }
        KeyCode::Backspace => {
            if app.command_cursor > 0 {
                let byte_pos = char_to_byte(&app.command_input, app.command_cursor - 1);
                let next_byte = char_to_byte(&app.command_input, app.command_cursor);
                app.command_input.drain(byte_pos..next_byte);
                app.command_cursor -= 1;
            }
            app.command_selected_index = 0;
            app.command_completion_query = None;
        }
        KeyCode::Delete | KeyCode::Char('d') if ctrl => {
            // Ctrl+D: delete char forward
            let len = app.command_input.chars().count();
            if app.command_cursor < len {
                let byte_pos = char_to_byte(&app.command_input, app.command_cursor);
                let next_byte = char_to_byte(&app.command_input, app.command_cursor + 1);
                app.command_input.drain(byte_pos..next_byte);
            }
        }
        KeyCode::Char('a') if ctrl => {
            app.command_cursor = 0;
        }
        KeyCode::Char('e') if ctrl => {
            app.command_cursor = app.command_input.chars().count();
        }
        KeyCode::Char('b') if ctrl => {
            if app.command_cursor > 0 {
                app.command_cursor -= 1;
            }
        }
        KeyCode::Char('f') if ctrl => {
            let len = app.command_input.chars().count();
            if app.command_cursor < len {
                app.command_cursor += 1;
            }
        }
        KeyCode::Char('b') if alt => {
            app.command_cursor = cmd_word_start_backward(&app.command_input, app.command_cursor);
        }
        KeyCode::Char('f') if alt => {
            app.command_cursor = cmd_word_end_forward(&app.command_input, app.command_cursor);
        }
        KeyCode::Char('k') if ctrl => {
            // Kill to end of line
            let byte_pos = char_to_byte(&app.command_input, app.command_cursor);
            app.command_input.truncate(byte_pos);
        }
        KeyCode::Char('u') if ctrl => {
            // Kill to beginning of line
            let byte_pos = char_to_byte(&app.command_input, app.command_cursor);
            app.command_input.drain(..byte_pos);
            app.command_cursor = 0;
        }
        KeyCode::Char('w') if ctrl => {
            cmd_kill_word_backward(app);
        }
        KeyCode::Left if alt => {
            app.command_cursor = cmd_word_start_backward(&app.command_input, app.command_cursor);
        }
        KeyCode::Right if alt => {
            app.command_cursor = cmd_word_end_forward(&app.command_input, app.command_cursor);
        }
        KeyCode::Left => {
            if app.command_cursor > 0 {
                app.command_cursor -= 1;
            }
        }
        KeyCode::Right => {
            let len = app.command_input.chars().count();
            if app.command_cursor < len {
                app.command_cursor += 1;
            }
        }
        KeyCode::Home => {
            app.command_cursor = 0;
        }
        KeyCode::End => {
            app.command_cursor = app.command_input.chars().count();
        }
        KeyCode::Char(c) if !ctrl && !alt => {
            let byte_pos = char_to_byte(&app.command_input, app.command_cursor);
            app.command_input.insert(byte_pos, c);
            app.command_cursor += 1;
            app.command_selected_index = 0;
            app.command_completion_query = None;
        }
        KeyCode::Up => {
            let hist_len = app.command_history.len();
            if hist_len > 0 {
                app.command_selected_index = (app.command_selected_index + 1).min(hist_len);
                let idx = hist_len - app.command_selected_index;
                if let Some(h) = app.command_history.get(idx) {
                    app.command_input = h.clone();
                    app.command_cursor = app.command_input.chars().count();
                }
            }
        }
        KeyCode::Down => {
            if app.command_selected_index > 0 {
                app.command_selected_index -= 1;
                if app.command_selected_index == 0 {
                    app.command_input.clear();
                    app.command_cursor = 0;
                } else {
                    let hist_len = app.command_history.len();
                    let idx = hist_len - app.command_selected_index;
                    if let Some(h) = app.command_history.get(idx) {
                        app.command_input = h.clone();
                        app.command_cursor = app.command_input.chars().count();
                    }
                }
            }
        }
        KeyCode::Tab | KeyCode::Char('n') if ctrl || key.code == KeyCode::Tab => {
            cmd_complete_next(app);
        }
        KeyCode::BackTab | KeyCode::Char('p') if ctrl || key.code == KeyCode::BackTab => {
            cmd_complete_prev(app);
        }
        _ => {}
    }
}

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

fn cmd_word_start_backward(s: &str, cursor: usize) -> usize {
    let chars: Vec<char> = s.chars().collect();
    let mut pos = cursor;
    // Skip whitespace
    while pos > 0 && chars[pos - 1].is_whitespace() {
        pos -= 1;
    }
    // Skip word chars
    while pos > 0 && !chars[pos - 1].is_whitespace() {
        pos -= 1;
    }
    pos
}

fn cmd_word_end_forward(s: &str, cursor: usize) -> usize {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut pos = cursor;
    // Skip whitespace
    while pos < len && chars[pos].is_whitespace() {
        pos += 1;
    }
    // Skip word chars
    while pos < len && !chars[pos].is_whitespace() {
        pos += 1;
    }
    pos
}

fn cmd_complete_next(app: &mut AppState) {
    let query = app
        .command_completion_query
        .get_or_insert_with(|| app.command_input.clone())
        .clone();
    let completions = crate::keybinds::commands::filter_commands(&query);
    let count = completions.len();
    if count == 0 {
        return;
    }
    let idx = app.command_selected_index.min(count - 1);
    if let Some(cmd) = completions.get(idx) {
        let base = cmd.name.split_whitespace().next().unwrap_or(cmd.name);
        app.command_input = base.to_string();
        app.command_cursor = app.command_input.chars().count();
        app.command_selected_index = (idx + 1) % count;
    }
}

fn cmd_complete_prev(app: &mut AppState) {
    let query = app
        .command_completion_query
        .get_or_insert_with(|| app.command_input.clone())
        .clone();
    let completions = crate::keybinds::commands::filter_commands(&query);
    let count = completions.len();
    if count == 0 {
        return;
    }
    let prev_idx = if app.command_selected_index == 0 {
        count - 1
    } else {
        app.command_selected_index - 1
    };
    if let Some(cmd) = completions.get(prev_idx) {
        let base = cmd.name.split_whitespace().next().unwrap_or(cmd.name);
        app.command_input = base.to_string();
        app.command_cursor = app.command_input.chars().count();
        app.command_selected_index = prev_idx;
    }
}

fn cmd_kill_word_backward(app: &mut AppState) {
    let new_cursor = cmd_word_start_backward(&app.command_input, app.command_cursor);
    let byte_start = char_to_byte(&app.command_input, new_cursor);
    let byte_end = char_to_byte(&app.command_input, app.command_cursor);
    app.command_input.drain(byte_start..byte_end);
    app.command_cursor = new_cursor;
    app.command_selected_index = 0;
}

fn get_google_credentials(app: &AppState) -> Option<(String, String)> {
    let id = app.config.google.client_id.clone()?;
    let secret = app.config.google.client_secret.clone()?;
    Some((id, secret))
}

async fn handle_search_input(app: &mut AppState, key: KeyEvent, _tx: &UnboundedSender<AppEvent>) {
    match key.code {
        KeyCode::Esc => {
            app.focus = FocusContext::Timeline;
            app.search_query.clear();
            app.search_results.clear();
        }
        KeyCode::Enter => {
            if let Some(event) = app.search_results.get(app.search_selected_index).cloned() {
                if let Some(ref dt) = event.start.date_time.clone() {
                    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(dt) {
                        let date = parsed.date_naive();
                        app.selected_day = date;
                        app.view_anchor_day = date;
                        app.selected_event_id = Some(event.id.clone());
                    }
                } else if let Some(ref d) = event.start.date.clone() {
                    if let Ok(date) = chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d") {
                        app.selected_day = date;
                        app.view_anchor_day = date;
                        app.selected_event_id = Some(event.id.clone());
                    }
                }
                app.focus = FocusContext::Timeline;
            }
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            let q = app.search_query.clone();
            app.search_events(&q);
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            let q = app.search_query.clone();
            app.search_events(&q);
        }
        KeyCode::Down => {
            if app.search_selected_index + 1 < app.search_results.len() {
                app.search_selected_index += 1;
            }
        }
        KeyCode::Up => {
            app.search_selected_index = app.search_selected_index.saturating_sub(1);
        }
        _ => {}
    }
}

fn cycle_focus(app: &mut AppState) {
    app.focus = match app.focus {
        FocusContext::Timeline => {
            if app.calendar_sidebar_visible {
                FocusContext::Calendars
            } else {
                FocusContext::Days
            }
        }
        FocusContext::Calendars => FocusContext::Days,
        FocusContext::Days => FocusContext::Timeline,
        _ => FocusContext::Timeline,
    };
}

fn open_event_dialog_new(app: &mut AppState) {
    let prev = app.focus.clone();
    app.dialog_state = Some(new_event_dialog_state(app));
    app.push_overlay(Overlay {
        kind: OverlayKind::Dialog,
        prev_focus: Some(prev),
    });
}

fn open_event_dialog_edit(app: &mut AppState) {
    let id = match app.selected_event_id.clone() {
        Some(id) => id,
        None => return,
    };
    let event = match app.events.get(&id) {
        Some(e) => e.clone(),
        None => return,
    };
    let prev = app.focus.clone();
    app.dialog_state = Some(DialogState::from_event(&event));
    app.push_overlay(Overlay {
        kind: OverlayKind::Dialog,
        prev_focus: Some(prev),
    });
}

fn toggle_calendar(app: &mut AppState) {
    if let Some(cal) = app.calendars.get(app.selected_calendar_index) {
        let key = cal.key.clone();
        if app.disabled_calendars.contains(&key) {
            app.disabled_calendars.remove(&key);
            app.config.hidden_calendars.remove(&key);
        } else {
            app.disabled_calendars.insert(key.clone());
            app.config.hidden_calendars.insert(key);
        }
        let _ = crate::config::loader::save_config(&app.config);
    }
}

fn select_new_event_calendar(app: &mut AppState) {
    if let Some(name) = app.select_new_event_calendar(app.selected_calendar_index) {
        let _ = crate::config::loader::save_config(&app.config);
        app.show_message(crate::state::message::Message::success(format!(
            "New events will use {}",
            name
        )));
    }
}

fn new_event_dialog_state(app: &AppState) -> DialogState {
    let mut ds = DialogState::new_event(app.selected_day);
    ds.calendar_key = app.selected_new_event_calendar_key();
    ds
}

fn execute_command(app: &mut AppState, cmd: &str, tx: &UnboundedSender<AppEvent>) {
    if !cmd.is_empty() {
        app.command_history.push(cmd.to_string());
    }
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    let verb = parts.first().copied().unwrap_or("");
    let arg = parts.get(1).copied().unwrap_or("").trim();

    match verb {
        "3" | "columns" => app.toggle_column_count(),
        "now" => app.jump_to_now(),
        "goto" | "g" => {
            if arg.is_empty() {
                let prev = app.focus.clone();
                app.goto_input.clear();
                app.goto_preview = None;
                app.push_overlay(Overlay {
                    kind: OverlayKind::Goto,
                    prev_focus: Some(prev),
                });
            } else if arg == "now" || arg == "today" {
                app.jump_to_now();
            } else if let Some(parsed) = parse_natural_date(arg) {
                app.selected_day = parsed.date;
                app.view_anchor_day = parsed.date;
                app.auto_select_event();
            }
        }
        "new" => {
            let prev = app.focus.clone();
            app.dialog_state = Some(new_event_dialog_state(app));
            if !arg.is_empty() {
                if let Some(ref mut ds) = app.dialog_state {
                    ds.title = arg.to_string();
                }
            }
            app.push_overlay(Overlay {
                kind: OverlayKind::Dialog,
                prev_focus: Some(prev),
            });
        }
        "edit" => open_event_dialog_edit(app),
        "delete" => initiate_delete(app),
        "messages" => {
            let prev = app.focus.clone();
            app.push_overlay(Overlay {
                kind: OverlayKind::Messages,
                prev_focus: Some(prev),
            });
        }
        "accounts" => {
            let prev = app.focus.clone();
            app.push_overlay(Overlay {
                kind: OverlayKind::Accounts,
                prev_focus: Some(prev),
            });
        }
        "sync" => {
            tx.send(AppEvent::TriggerSync).ok();
        }
        "resync" => {
            cmd_resync(app, tx);
        }
        "login" => cmd_login(app, tx),
        "logout" => cmd_logout(app, tx),
        "caldav" => cmd_caldav(app),
        "notifications" => {
            let prev = app.focus.clone();
            app.notifications_selected_index = 0;
            app.push_overlay(Overlay {
                kind: OverlayKind::Notifications,
                prev_focus: Some(prev),
            });
        }
        "meet" | "meetwith" => {
            let contacts = crate::api::contacts::merged_contacts(&app.events);
            let prev = app.focus.clone();
            let mut mw = crate::state::meet_with::MeetWithState::new(contacts);
            // Prefill email if arg given
            if !arg.is_empty() {
                mw.search_query = arg.to_string();
            }
            app.meet_with = Some(mw);
            app.push_overlay(Overlay {
                kind: OverlayKind::MeetWith,
                prev_focus: Some(prev),
            });
        }
        "propose" => {
            if let Some(ref id) = app.selected_event_id.clone() {
                if let Some(event) = app.events.get(id).cloned() {
                    open_propose_time_dialog(app, &event);
                }
            }
        }
        "quit" | "q" => {
            app.focus = FocusContext::Timeline;
        }
        _ => {}
    }
}

fn handle_login_prompt_input(app: &mut AppState, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.pop_overlay();
            cmd_login(app, tx);
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.pop_overlay();
        }
        _ => {}
    }
}

fn cmd_login(app: &mut AppState, tx: &UnboundedSender<AppEvent>) {
    let (client_id, client_secret) = match get_google_credentials(app) {
        Some(creds) => creds,
        None => {
            app.show_message(crate::state::message::Message::error(
                "Google credentials not configured. Add google.clientId/clientSecret to ~/.config/aion/config.toml"
            ));
            return;
        }
    };

    if app.is_auth_loading {
        return; // already in progress
    }
    app.is_auth_loading = true;

    let tx_url = tx.clone();
    let tx_result = tx.clone();
    tokio::spawn(async move {
        let on_url = move |url: String| {
            tx_url.send(AppEvent::AuthUrlReady(url)).ok();
        };
        match crate::auth::oauth::start_login_flow(client_id, client_secret, on_url).await {
            Ok(result) => {
                tx_result.send(AppEvent::AuthSuccess(result.account)).ok();
            }
            Err(e) => {
                tx_result.send(AppEvent::AuthError(e.to_string())).ok();
            }
        }
    });
}

fn cmd_resync(app: &mut AppState, tx: &UnboundedSender<AppEvent>) {
    match crate::db::connection::open_connection() {
        Ok(conn) => {
            let _ = crate::db::events_repo::delete_all_events(&conn);
        }
        Err(e) => {
            app.show_message(crate::state::message::Message::error(format!(
                "Failed to clear events DB: {e}"
            )));
            return;
        }
    }
    let _ = crate::db::sync_tokens::clear_all_sync_tokens();
    app.events.clear();
    app.show_message(crate::state::message::Message::info(
        "Cleared local cache — resyncing from remote…",
    ));
    tx.send(AppEvent::TriggerSync).ok();
}

fn cmd_logout(app: &mut AppState, _tx: &UnboundedSender<AppEvent>) {
    let accounts = crate::auth::tokens::get_accounts();
    if accounts.is_empty() {
        app.show_message(crate::state::message::Message::info(
            "No accounts to log out from.",
        ));
        return;
    }
    for a in &accounts {
        let _ = crate::auth::tokens::remove_account(&a.account.email);
    }
    let _ = crate::db::sync_tokens::clear_all_sync_tokens();
    app.reload_accounts();
    app.events.clear();
    app.calendars.clear();
    app.is_logged_in = false;
    app.show_message(crate::state::message::Message::success(
        "Logged out from all accounts.",
    ));
}

fn handle_dialog_input(app: &mut AppState, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
    use crate::state::dialog::{F_ALL_DAY, F_CALENDAR, F_EVENT_TYPE, F_WHEN};
    let ctrl = key.modifiers == KeyModifiers::CONTROL;
    let shift = key.modifiers == KeyModifiers::SHIFT;
    let no_mods = key.modifiers == KeyModifiers::NONE;

    match key.code {
        KeyCode::Esc => {
            app.dialog_state = None;
            app.pop_overlay();
        }

        // Ctrl+S: save
        KeyCode::Char('s') if ctrl => {
            dialog_save(app, tx);
        }

        // Tab: next field
        KeyCode::Tab => {
            if let Some(ref mut ds) = app.dialog_state {
                ds.next_field();
            }
        }

        // Shift+Tab: prev field
        KeyCode::BackTab => {
            if let Some(ref mut ds) = app.dialog_state {
                ds.prev_field();
            }
        }

        KeyCode::Down if no_mods => {
            if app
                .dialog_state
                .as_ref()
                .is_some_and(|ds| ds.active_field == F_CALENDAR)
            {
                cycle_dialog_calendar(app, 1);
            }
        }

        KeyCode::Up if no_mods => {
            if app
                .dialog_state
                .as_ref()
                .is_some_and(|ds| ds.active_field == F_CALENDAR)
            {
                cycle_dialog_calendar(app, -1);
            }
        }

        KeyCode::Char('j') if no_mods => {
            if app
                .dialog_state
                .as_ref()
                .is_some_and(|ds| ds.active_field == F_CALENDAR)
            {
                cycle_dialog_calendar(app, 1);
            } else if let Some(ref mut ds) = app.dialog_state {
                ds.push_char('j');
            }
        }

        KeyCode::Char('k') if no_mods => {
            if app
                .dialog_state
                .as_ref()
                .is_some_and(|ds| ds.active_field == F_CALENDAR)
            {
                cycle_dialog_calendar(app, -1);
            } else if let Some(ref mut ds) = app.dialog_state {
                ds.push_char('k');
            }
        }

        // Backspace
        KeyCode::Backspace if no_mods => {
            if let Some(ref mut ds) = app.dialog_state {
                ds.pop_char();
            }
        }

        // Enter: apply when-input if in F_WHEN, else next field
        KeyCode::Enter if no_mods => {
            if let Some(ref mut ds) = app.dialog_state {
                if ds.active_field == F_WHEN && !ds.when_input.is_empty() {
                    ds.apply_when_input();
                } else if ds.active_field == F_CALENDAR {
                    ds.next_field();
                } else if ds.active_field == F_EVENT_TYPE {
                    ds.event_type_idx =
                        (ds.event_type_idx + 1) % crate::state::dialog::EVENT_TYPE_LABELS.len();
                } else if ds.active_field == F_ALL_DAY {
                    ds.toggle_all_day();
                } else {
                    ds.next_field();
                }
            }
        }

        // Space: toggle AllDay or cycle EventType
        KeyCode::Char(' ') if no_mods => {
            if let Some(ref mut ds) = app.dialog_state {
                if ds.active_field == F_ALL_DAY {
                    ds.toggle_all_day();
                } else if ds.active_field == F_CALENDAR {
                    cycle_dialog_calendar(app, 1);
                } else if ds.active_field == F_EVENT_TYPE {
                    ds.event_type_idx =
                        (ds.event_type_idx + 1) % crate::state::dialog::EVENT_TYPE_LABELS.len();
                } else {
                    ds.push_char(' ');
                }
            }
        }

        // Regular character input
        KeyCode::Char(c) if no_mods || shift => {
            if let Some(ref mut ds) = app.dialog_state {
                ds.push_char(c);
            }
        }

        _ => {}
    }
}

fn cycle_dialog_calendar(app: &mut AppState, delta: isize) {
    let default_calendar_key = app.selected_new_event_calendar_key();
    let Some(ds) = app.dialog_state.as_mut() else {
        return;
    };
    if app.calendars.is_empty() {
        ds.calendar_key = None;
        return;
    }

    let current_key = ds
        .calendar_key
        .as_deref()
        .or(default_calendar_key.as_deref());
    let current_idx = current_key
        .and_then(|key| app.calendars.iter().position(|cal| cal.key == key))
        .unwrap_or(0);
    let len = app.calendars.len() as isize;
    let next_idx = (current_idx as isize + delta).rem_euclid(len) as usize;
    ds.calendar_key = Some(app.calendars[next_idx].key.clone());
}

fn dialog_save(app: &mut AppState, tx: &UnboundedSender<AppEvent>) {
    let (event, persist_default_key) = match app.dialog_state.as_ref() {
        Some(ds) => (
            ds.build_event(),
            if ds.is_edit {
                None
            } else {
                ds.calendar_key.clone()
            },
        ),
        None => return,
    };
    if let Some(key) = persist_default_key {
        app.select_new_event_calendar_by_key(&key);
        let _ = crate::config::loader::save_config(&app.config);
    }

    if !event.id.starts_with("local-") {
        let id = event.id.clone();
        app.events.insert(id.clone(), event);
        app.selected_event_id = Some(id);
        app.rebuild_calendars_pub();
        app.dialog_state = None;
        app.pop_overlay();
        app.auto_select_event();
        return;
    }

    let account_email = match event.account_email.clone() {
        Some(email) if !email.is_empty() => email,
        _ => {
            app.show_message(crate::state::message::Message::error(
                "Choose a calendar before creating the event.",
            ));
            return;
        }
    };
    let calendar_id = match event.calendar_id.clone() {
        Some(id) if !id.is_empty() => id,
        _ => {
            app.show_message(crate::state::message::Message::error(
                "Choose a calendar before creating the event.",
            ));
            return;
        }
    };

    let account_type = app
        .accounts
        .iter()
        .find(|account| account.email == account_email)
        .map(|account| account.account_type.clone())
        .or_else(|| {
            if app
                .config
                .caldav
                .iter()
                .any(|account| account.email == account_email)
            {
                Some(crate::auth::tokens::AccountType::CalDAV)
            } else {
                Some(crate::auth::tokens::AccountType::Google)
            }
        });
    let config = app.config.clone();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let result = match account_type {
            Some(crate::auth::tokens::AccountType::CalDAV) => {
                let account = config
                    .caldav
                    .iter()
                    .find(|account| account.email == account_email)
                    .cloned();
                match account {
                    Some(account) => {
                        crate::api::caldav::create_caldav_event(&account, &calendar_id, &event)
                            .await
                    }
                    None => Err(anyhow::anyhow!(
                        "CalDAV account not found for {}",
                        account_email
                    )),
                }
            }
            Some(crate::auth::tokens::AccountType::Google) | None => {
                let client_id = config.google.client_id.clone().unwrap_or_default();
                let client_secret = config.google.client_secret.clone().unwrap_or_default();
                if client_id.is_empty() || client_secret.is_empty() {
                    Err(anyhow::anyhow!("Google credentials not configured."))
                } else {
                    let client = crate::api::gcal::GcalClient::new(client_id, client_secret);
                    client
                        .create_event(&account_email, &calendar_id, &event)
                        .await
                }
            }
        };

        match result {
            Ok(created) => {
                tx_clone.send(AppEvent::EventCreated(created)).ok();
            }
            Err(e) => {
                tx_clone
                    .send(AppEvent::EventCreateError(e.to_string()))
                    .ok();
            }
        }
    });

    app.show_message(crate::state::message::Message::info(
        "Creating event on calendar…",
    ));
    app.dialog_state = None;
    app.pop_overlay();
}

fn handle_goto_input(app: &mut AppState, key: KeyEvent) {
    let no_mods = key.modifiers == KeyModifiers::NONE;

    match key.code {
        KeyCode::Esc => {
            app.goto_input.clear();
            app.goto_preview = None;
            app.pop_overlay();
        }

        KeyCode::Enter if no_mods => {
            // Navigate to parsed date
            if let Some(parsed) = parse_natural_date(&app.goto_input.clone()) {
                app.selected_day = parsed.date;
                app.view_anchor_day = parsed.date;
                app.focused_column = 0;
                app.auto_select_event();
            }
            app.goto_input.clear();
            app.goto_preview = None;
            app.pop_overlay();
        }

        KeyCode::Backspace if no_mods => {
            app.goto_input.pop();
            refresh_goto_preview(app);
        }

        KeyCode::Char(c) if no_mods => {
            app.goto_input.push(c);
            refresh_goto_preview(app);
        }

        _ => {}
    }
}

fn refresh_goto_preview(app: &mut AppState) {
    if app.goto_input.is_empty() {
        app.goto_preview = None;
        return;
    }
    app.goto_preview = parse_natural_date(&app.goto_input.clone())
        .map(|p| crate::domain::natural_date::format_parsed_preview(&p));
}

fn handle_messages_input(app: &mut AppState, key: KeyEvent) {
    let no_mods = key.modifiers == KeyModifiers::NONE;

    match key.code {
        KeyCode::Esc => {
            app.pop_overlay();
        }
        KeyCode::Char('c') if no_mods => {
            app.message_log.clear();
        }
        _ => {}
    }
}

fn cmd_caldav(app: &mut AppState) {
    use crate::state::caldav_dialog::CalDavDialogState;
    let prev = app.focus.clone();
    app.caldav_dialog = Some(CalDavDialogState::new());
    app.push_overlay(crate::state::overlay::Overlay {
        kind: OverlayKind::CalDavLogin,
        prev_focus: Some(prev),
    });
}

async fn handle_caldav_login_input(
    app: &mut AppState,
    key: KeyEvent,
    tx: &UnboundedSender<AppEvent>,
) {
    let ctrl = key.modifiers == KeyModifiers::CONTROL;
    let shift = key.modifiers == KeyModifiers::SHIFT;
    let no_mods = key.modifiers == KeyModifiers::NONE;

    match key.code {
        KeyCode::Esc => {
            app.caldav_dialog = None;
            app.pop_overlay();
        }

        KeyCode::Tab => {
            if let Some(ref mut cd) = app.caldav_dialog {
                cd.next_field();
            }
        }

        KeyCode::BackTab => {
            if let Some(ref mut cd) = app.caldav_dialog {
                cd.prev_field();
            }
        }

        KeyCode::Backspace if no_mods => {
            if let Some(ref mut cd) = app.caldav_dialog {
                cd.delete_char();
            }
        }

        // 1-6: select preset
        KeyCode::Char(c) if no_mods && ('1'..='6').contains(&c) => {
            let idx = c as usize - '1' as usize;
            if let Some(ref mut cd) = app.caldav_dialog {
                cd.apply_preset(idx);
            }
        }

        // t: test connection
        KeyCode::Char('t') if no_mods => {
            if let Some(ref cd) = app.caldav_dialog {
                let account = cd.to_config_entry();
                let tx_clone = tx.clone();
                app.caldav_dialog.as_mut().unwrap().testing = true;
                app.caldav_dialog.as_mut().unwrap().test_result = None;
                tokio::spawn(async move {
                    let result = crate::api::caldav::test_caldav_connection(&account).await;
                    let msg = match result {
                        Ok(msg) => AppEvent::CalDavTestResult(Ok(msg)),
                        Err(e) => AppEvent::CalDavTestResult(Err(e.to_string())),
                    };
                    tx_clone.send(msg).ok();
                });
            }
        }

        // Ctrl+S: save
        KeyCode::Char('s') if ctrl => {
            caldav_dialog_save(app, tx);
        }

        // Enter: next field
        KeyCode::Enter if no_mods => {
            if let Some(ref mut cd) = app.caldav_dialog {
                cd.next_field();
            }
        }

        KeyCode::Char(c) if no_mods || shift => {
            if let Some(ref mut cd) = app.caldav_dialog {
                cd.insert_char(c);
            }
        }

        _ => {}
    }
}

fn caldav_dialog_save(app: &mut AppState, tx: &UnboundedSender<AppEvent>) {
    let entry = match app.caldav_dialog.as_ref() {
        Some(cd) => {
            if !cd.is_valid() {
                app.show_message(crate::state::message::Message::error(
                    "Email, Server URL, Username, and a password are required.",
                ));
                return;
            }
            cd.to_config_entry()
        }
        None => return,
    };
    app.caldav_dialog.as_mut().unwrap().saving = true;

    match crate::auth::tokens::save_caldav_account_to_store(
        &entry.email,
        &entry.name,
        &entry.server_url,
        &entry.username,
        entry.password.as_deref(),
        entry.password_command.as_deref(),
    ) {
        Ok(()) => {
            app.caldav_dialog = None;
            app.pop_overlay();
            app.reload_accounts();
            let email = entry.email.clone();
            app.show_message(crate::state::message::Message::success(format!(
                "CalDAV account {} saved. Run ':sync' to fetch events.",
                email
            )));
            tx.send(AppEvent::TriggerSync).ok();
        }
        Err(e) => {
            app.caldav_dialog.as_mut().unwrap().saving = false;
            app.show_message(crate::state::message::Message::error(format!(
                "Failed to save: {}",
                e
            )));
        }
    }
}

fn handle_accounts_input(app: &mut AppState, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
    let no_mods = key.modifiers == KeyModifiers::NONE;

    if app.accounts_delete_confirm {
        match key.code {
            KeyCode::Char('y') if no_mods => {
                if let Some(account) = app.accounts.get(app.accounts_selected_index).cloned() {
                    let _ = crate::auth::tokens::remove_account(&account.email);
                    let _ = crate::db::sync_tokens::clear_all_sync_tokens();
                    app.reload_accounts();
                    app.events
                        .retain(|_, e| e.account_email.as_deref() != Some(&account.email));
                    app.rebuild_calendars_pub();
                    if app.accounts.is_empty() {
                        app.is_logged_in = false;
                        app.pop_overlay();
                    }
                    app.show_message(crate::state::message::Message::success(format!(
                        "Removed {}",
                        account.email
                    )));
                }
                app.accounts_delete_confirm = false;
            }
            KeyCode::Char('n') | KeyCode::Esc if no_mods => {
                app.accounts_delete_confirm = false;
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.pop_overlay();
        }
        KeyCode::Char('j') | KeyCode::Down if no_mods => {
            if app.accounts_selected_index + 1 < app.accounts.len() {
                app.accounts_selected_index += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up if no_mods => {
            app.accounts_selected_index = app.accounts_selected_index.saturating_sub(1);
        }
        KeyCode::Char('d') if no_mods => {
            if !app.accounts.is_empty() {
                app.accounts_delete_confirm = true;
            }
        }
        KeyCode::Char('p') if no_mods => {
            if let Some(account) = app.accounts.get(app.accounts_selected_index).cloned() {
                let mut store = crate::auth::tokens::load_accounts_store();
                store.default_account = Some(account.email.clone());
                let _ = crate::auth::tokens::save_accounts_store(&store);
                app.show_message(crate::state::message::Message::success(format!(
                    "{} is now the primary account",
                    account.email
                )));
            }
        }
        _ => {}
    }
}

// ── Phase 5: Helpers ──────────────────────────────────────────────────────────

fn rsvp_from_details(app: &mut AppState, status: &str, tx: &UnboundedSender<AppEvent>) {
    let id = match app.selected_event_id.clone() {
        Some(id) => id,
        None => return,
    };

    let is_recurring = app.events.get(&id).map_or(false, |e| e.is_recurring());

    if is_recurring {
        app.pending_rsvp_status = Some(status.to_string());
        app.recurrence_scope = RecurrenceScope::This;
        let prev = app.focus.clone();
        app.push_overlay(Overlay {
            kind: OverlayKind::Confirm,
            prev_focus: Some(prev),
        });
        return;
    }

    apply_rsvp_single(app, &id, status, tx);
}

fn apply_rsvp_single(app: &mut AppState, id: &str, status: &str, tx: &UnboundedSender<AppEvent>) {
    use crate::domain::event::ResponseStatus;
    let rs = match status {
        "accepted" => ResponseStatus::Accepted,
        "declined" => ResponseStatus::Declined,
        "tentative" => ResponseStatus::Tentative,
        _ => ResponseStatus::NeedsAction,
    };
    if let Some(ev) = app.events.get_mut(id) {
        if let Some(ref mut attendees_mut) = ev.attendees {
            for a in attendees_mut.iter_mut() {
                if a.is_self == Some(true) {
                    a.response_status = Some(rs.clone());
                }
            }
        }
    }

    // Eagerly push the RSVP to the API, then trigger a sync to confirm.
    if let Some(ev) = app.events.get(id) {
        if let (Some(account_email), Some(calendar_id)) =
            (ev.account_email.clone(), ev.calendar_id.clone())
        {
            let event_id = id.to_owned();
            let status_str = status.to_owned();
            let attendees = ev.attendees.clone().unwrap_or_default();
            let tx_clone = tx.clone();
            let config = crate::config::loader::load_config();
            let client_id = config.google.client_id.clone().unwrap_or_default();
            let client_secret = config.google.client_secret.clone().unwrap_or_default();
            tokio::spawn(async move {
                let gcal = crate::api::gcal::GcalClient::new(client_id, client_secret);
                match gcal
                    .update_attendance(
                        &account_email,
                        &calendar_id,
                        &event_id,
                        &status_str,
                        &attendees,
                    )
                    .await
                {
                    Ok(()) => {
                        tx_clone
                            .send(AppEvent::RsvpComplete {
                                event_id: event_id.clone(),
                            })
                            .ok();
                        tx_clone.send(AppEvent::TriggerSync).ok();
                    }
                    Err(e) => {
                        tracing::warn!(
                            "RSVP update failed for {}/{} {}: {}",
                            account_email,
                            calendar_id,
                            event_id,
                            e
                        );
                        tx_clone.send(AppEvent::RsvpError(e.to_string())).ok();
                    }
                }
            });
        }
    }

    app.show_message(crate::state::message::Message::info(format!(
        "RSVP: {} (syncing…)",
        status
    )));
}

fn apply_rsvp(
    app: &mut AppState,
    id: &str,
    scope: &RecurrenceScope,
    status: &str,
    tx: &UnboundedSender<AppEvent>,
) {
    use crate::domain::event::ResponseStatus;
    let rs = match status {
        "accepted" => ResponseStatus::Accepted,
        "declined" => ResponseStatus::Declined,
        "tentative" => ResponseStatus::Tentative,
        _ => ResponseStatus::NeedsAction,
    };

    match scope {
        RecurrenceScope::This => {
            apply_rsvp_single(app, id, status, tx);
        }
        RecurrenceScope::Following | RecurrenceScope::All => {
            let recurring_id = app
                .events
                .get(id)
                .and_then(|e| e.recurring_event_id.clone());
            let base_start = app
                .events
                .get(id)
                .and_then(|e| e.start.date_time.clone().or(e.start.date.clone()));

            if let Some(ref rid) = recurring_id {
                let rid = rid.clone();
                let cutoff = base_start.unwrap_or_default();
                let ids_to_update: Vec<String> = app
                    .events
                    .values()
                    .filter(|e| {
                        if e.recurring_event_id.as_deref() == Some(&rid) || e.id == id {
                            let e_start = e
                                .start
                                .date_time
                                .as_deref()
                                .or(e.start.date.as_deref())
                                .unwrap_or("");
                            e_start >= cutoff.as_str()
                        } else {
                            false
                        }
                    })
                    .map(|e| e.id.clone())
                    .collect();

                for eid in ids_to_update {
                    apply_rsvp_single(app, &eid, status, tx);
                }
                // Override the per-event messages with a scoped summary
                let label = match rs {
                    ResponseStatus::Accepted => "accepted",
                    ResponseStatus::Declined => "declined",
                    ResponseStatus::Tentative => "tentative",
                    ResponseStatus::NeedsAction => "awaiting",
                };
                app.show_message(crate::state::message::Message::info(format!(
                    "RSVP: {} for this and all future events (syncing…)",
                    label
                )));
            } else {
                apply_rsvp_single(app, id, status, tx);
            }
        }
    }
}

fn open_propose_time_dialog(app: &mut AppState, event: &crate::domain::event::CalEvent) {
    use crate::state::propose_time::ProposeTimeState;
    let prev = app.focus.clone();
    app.propose_time = Some(ProposeTimeState::from_event(event));
    app.push_overlay(Overlay {
        kind: OverlayKind::ProposeTime,
        prev_focus: Some(prev),
    });
}

// ── Phase 5: Notifications overlay ───────────────────────────────────────────

async fn handle_notifications_input(
    app: &mut AppState,
    key: KeyEvent,
    tx: &UnboundedSender<AppEvent>,
) {
    let no_mods = key.modifiers == KeyModifiers::NONE;

    match key.code {
        KeyCode::Esc => app.pop_overlay(),

        KeyCode::Char('j') | KeyCode::Down if no_mods => {
            let count = app.pending_invites().len();
            if count > 0 {
                app.notifications_selected_index =
                    (app.notifications_selected_index + 1).min(count - 1);
            }
        }

        KeyCode::Char('k') | KeyCode::Up if no_mods => {
            app.notifications_selected_index = app.notifications_selected_index.saturating_sub(1);
        }

        KeyCode::Char('y') if no_mods => rsvp_selected_notification(app, "accepted", tx),
        KeyCode::Char('n') if no_mods => rsvp_selected_notification(app, "declined", tx),
        KeyCode::Char('m') if no_mods => rsvp_selected_notification(app, "tentative", tx),

        _ => {}
    }
}

fn rsvp_selected_notification(app: &mut AppState, status: &str, tx: &UnboundedSender<AppEvent>) {
    let invites: Vec<String> = app
        .pending_invites()
        .into_iter()
        .map(|e| e.id.clone())
        .collect();

    let idx = app
        .notifications_selected_index
        .min(invites.len().saturating_sub(1));
    if let Some(id) = invites.get(idx).cloned() {
        app.selected_event_id = Some(id.clone());

        let is_recurring = app.events.get(&id).map_or(false, |e| e.is_recurring());
        if is_recurring {
            // Push recurrence scope confirm on top of notifications overlay
            app.pending_rsvp_status = Some(status.to_string());
            app.recurrence_scope = RecurrenceScope::This;
            let prev = app.focus.clone();
            app.push_overlay(Overlay {
                kind: OverlayKind::Confirm,
                prev_focus: Some(prev),
            });
            return;
        }

        apply_rsvp_single(app, &id, status, tx);
        // Clamp index if list shrinks
        let new_count = app.pending_invites().len();
        if new_count == 0 {
            app.pop_overlay();
        } else {
            app.notifications_selected_index = app
                .notifications_selected_index
                .min(new_count.saturating_sub(1));
        }
    }
}

// ── Phase 5: ProposeTime dialog ───────────────────────────────────────────────

async fn handle_propose_time_input(
    app: &mut AppState,
    key: KeyEvent,
    _tx: &UnboundedSender<AppEvent>,
) {
    use crate::state::propose_time::PT_FIELD_WHEN;
    let ctrl = key.modifiers == KeyModifiers::CONTROL;
    let shift = key.modifiers == KeyModifiers::SHIFT;
    let no_mods = key.modifiers == KeyModifiers::NONE;

    match key.code {
        KeyCode::Esc => {
            app.propose_time = None;
            app.pop_overlay();
        }

        KeyCode::Tab => {
            if let Some(ref mut pt) = app.propose_time {
                pt.next_field();
            }
        }

        KeyCode::BackTab => {
            if let Some(ref mut pt) = app.propose_time {
                pt.prev_field();
            }
        }

        KeyCode::Enter if no_mods => {
            let should_apply = app.propose_time.as_ref().map_or(false, |pt| {
                pt.active_field == PT_FIELD_WHEN
                    && !pt.when_input.is_empty()
                    && pt.when_preview.is_some()
            });
            if should_apply {
                if let Some(ref mut pt) = app.propose_time {
                    pt.apply_when_input();
                }
            } else if let Some(ref mut pt) = app.propose_time {
                pt.next_field();
            }
        }

        KeyCode::Char('s') if ctrl => propose_time_send(app),

        KeyCode::Backspace if no_mods => {
            if let Some(ref mut pt) = app.propose_time {
                pt.pop_char();
            }
        }

        KeyCode::Char(c) if no_mods || shift => {
            if let Some(ref mut pt) = app.propose_time {
                pt.push_char(c);
            }
        }

        _ => {}
    }
}

fn propose_time_send(app: &mut AppState) {
    app.propose_time = None;
    app.pop_overlay();
    app.show_message(crate::state::message::Message::info(
        "Time proposal noted. In a future release this will notify the organizer.",
    ));
}

// ── Phase 5: MeetWith dialog ──────────────────────────────────────────────────

async fn handle_meet_with_input(app: &mut AppState, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
    let no_mods = key.modifiers == KeyModifiers::NONE;
    let ctrl = key.modifiers == KeyModifiers::CONTROL;
    let shift = key.modifiers == KeyModifiers::SHIFT;

    let step = app.meet_with.as_ref().map(|m| m.step.clone());

    match step {
        Some(crate::state::meet_with::MeetWithStep::People) => {
            meet_with_people_input(app, key, tx, no_mods, ctrl, shift).await;
        }
        Some(crate::state::meet_with::MeetWithStep::Slots) => {
            meet_with_slots_input(app, key, tx, no_mods, ctrl).await;
        }
        None => {
            if key.code == KeyCode::Esc {
                app.meet_with = None;
                app.pop_overlay();
            }
        }
    }
}

async fn meet_with_people_input(
    app: &mut AppState,
    key: KeyEvent,
    tx: &UnboundedSender<AppEvent>,
    no_mods: bool,
    ctrl: bool,
    shift: bool,
) {
    match key.code {
        KeyCode::Esc => {
            app.meet_with = None;
            app.pop_overlay();
        }

        KeyCode::Up | KeyCode::Char('k') if no_mods => {
            if let Some(ref mut mw) = app.meet_with {
                mw.list_cursor = mw.list_cursor.saturating_sub(1);
            }
        }

        KeyCode::Down | KeyCode::Char('j') if no_mods => {
            if let Some(ref mut mw) = app.meet_with {
                let count = mw.filtered_contacts().len();
                if count > 0 {
                    mw.list_cursor = (mw.list_cursor + 1).min(count - 1);
                }
            }
        }

        KeyCode::Tab | KeyCode::Enter if no_mods => {
            if let Some(ref mut mw) = app.meet_with {
                let query = mw.search_query.clone();
                let contacts: Vec<String> = mw
                    .filtered_contacts()
                    .iter()
                    .map(|c| c.email.clone())
                    .collect();
                if query.contains('@') {
                    mw.add_email(&query);
                    mw.search_query.clear();
                    mw.list_cursor = 0;
                } else if let Some(email) = contacts.get(mw.list_cursor).cloned() {
                    mw.toggle_contact(&email);
                }
            }
        }

        // Ctrl+Right or Ctrl+N: go to slots
        KeyCode::Right if ctrl => advance_to_slots(app, tx).await,
        KeyCode::Char('n') if ctrl => advance_to_slots(app, tx).await,

        KeyCode::Backspace if no_mods => {
            if let Some(ref mut mw) = app.meet_with {
                mw.search_query.pop();
                mw.list_cursor = 0;
            }
        }

        KeyCode::Char(c) if no_mods || shift => {
            if let Some(ref mut mw) = app.meet_with {
                mw.search_query.push(c);
                mw.list_cursor = 0;
            }
        }

        _ => {}
    }
}

async fn advance_to_slots(app: &mut AppState, tx: &UnboundedSender<AppEvent>) {
    let (emails, duration) = match app.meet_with.as_ref() {
        Some(mw) if !mw.selected_people.is_empty() => (mw.selected_emails(), mw.duration_minutes),
        Some(_) => {
            app.show_message(crate::state::message::Message::error(
                "Select at least one person first.",
            ));
            return;
        }
        None => return,
    };

    if let Some(ref mut mw) = app.meet_with {
        mw.step = crate::state::meet_with::MeetWithStep::Slots;
        mw.slots_loading = true;
        mw.slots.clear();
        mw.slots_cursor = 0;
    }

    let account_email = app.accounts.first().map(|a| a.email.clone());
    let client_id = app.config.google.client_id.clone();
    let client_secret = app.config.google.client_secret.clone();
    let tx_clone = tx.clone();

    tokio::spawn(async move {
        let Some(account_email) = account_email else {
            tx_clone
                .send(AppEvent::FreeBusyError(
                    "No Google account configured".to_string(),
                ))
                .ok();
            return;
        };
        let (Some(client_id), Some(client_secret)) = (client_id, client_secret) else {
            tx_clone
                .send(AppEvent::FreeBusyError(
                    "Google credentials not configured".to_string(),
                ))
                .ok();
            return;
        };

        let now = chrono::Local::now();
        let time_min = now.to_rfc3339();
        let time_max = (now + chrono::Duration::days(7)).to_rfc3339();

        let client = crate::api::gcal::GcalClient::new(client_id, client_secret);
        let mut all_emails = emails.clone();
        if !all_emails.contains(&account_email) {
            all_emails.push(account_email.clone());
        }

        match client
            .query_free_busy(&account_email, &all_emails, &time_min, &time_max)
            .await
        {
            Ok(by_person) => {
                let combined = crate::domain::free_slots::combine_busy_periods(&by_person);
                let opts = crate::domain::free_slots::FindSlotsOptions {
                    min_duration_minutes: duration,
                    ..Default::default()
                };
                let slots = crate::domain::free_slots::find_meeting_slots(&combined, &opts);
                tx_clone.send(AppEvent::FreeBusyLoaded(slots)).ok();
            }
            Err(e) => {
                tx_clone.send(AppEvent::FreeBusyError(e.to_string())).ok();
            }
        }
    });
}

async fn meet_with_slots_input(
    app: &mut AppState,
    key: KeyEvent,
    tx: &UnboundedSender<AppEvent>,
    no_mods: bool,
    _ctrl: bool,
) {
    match key.code {
        KeyCode::Char('b') if no_mods => {
            if let Some(ref mut mw) = app.meet_with {
                mw.step = crate::state::meet_with::MeetWithStep::People;
                mw.slots.clear();
                mw.slots_cursor = 0;
            }
        }

        KeyCode::Esc => {
            app.meet_with = None;
            app.pop_overlay();
        }

        KeyCode::Up | KeyCode::Char('k') if no_mods => {
            if let Some(ref mut mw) = app.meet_with {
                mw.slots_cursor = mw.slots_cursor.saturating_sub(1);
            }
        }

        KeyCode::Down | KeyCode::Char('j') if no_mods => {
            if let Some(ref mut mw) = app.meet_with {
                let count = mw.slots.len();
                if count > 0 {
                    mw.slots_cursor = (mw.slots_cursor + 1).min(count - 1);
                }
            }
        }

        KeyCode::Left | KeyCode::Char('h') if no_mods => {
            if let Some(ref mut mw) = app.meet_with {
                mw.cycle_duration(false);
            }
            advance_to_slots(app, tx).await;
        }

        KeyCode::Right | KeyCode::Char('l') if no_mods => {
            if let Some(ref mut mw) = app.meet_with {
                mw.cycle_duration(true);
            }
            advance_to_slots(app, tx).await;
        }

        KeyCode::Enter if no_mods => create_meeting_from_slot(app, tx).await,

        _ => {}
    }
}

async fn create_meeting_from_slot(app: &mut AppState, tx: &UnboundedSender<AppEvent>) {
    let mw = match app.meet_with.as_ref() {
        Some(mw) if !mw.slots.is_empty() => mw,
        _ => return,
    };

    let slot_idx = mw.slots_cursor.min(mw.slots.len() - 1);
    let slot = mw.slots[slot_idx].clone();
    let people_names = mw.selected_names_display();
    let attendees: Vec<crate::domain::event::Attendee> = mw
        .selected_people
        .iter()
        .map(|p| crate::domain::event::Attendee {
            email: p.email.clone(),
            display_name: p.display_name.clone(),
            response_status: None,
            organizer: None,
            is_self: None,
        })
        .collect();

    let account_email = match app.accounts.first().map(|a| a.email.clone()) {
        Some(e) => e,
        None => {
            app.show_message(crate::state::message::Message::error("No Google account."));
            return;
        }
    };

    let client_id = match app.config.google.client_id.clone() {
        Some(id) => id,
        None => {
            app.show_message(crate::state::message::Message::error(
                "Google credentials not configured.",
            ));
            return;
        }
    };
    let client_secret = match app.config.google.client_secret.clone() {
        Some(s) => s,
        None => {
            app.show_message(crate::state::message::Message::error(
                "Google credentials not configured.",
            ));
            return;
        }
    };

    let tz_offset = chrono::Local::now().offset().to_string();
    let event = crate::domain::event::CalEvent {
        id: String::new(),
        summary: format!("Meeting with {}", people_names),
        description: None,
        location: None,
        html_link: None,
        status: crate::domain::event::EventStatus::Confirmed,
        event_type: None,
        start: crate::domain::event::TimeObject {
            date: None,
            date_time: Some(slot.start.to_rfc3339()),
            time_zone: Some(tz_offset.clone()),
        },
        end: crate::domain::event::TimeObject {
            date: None,
            date_time: Some(slot.end.to_rfc3339()),
            time_zone: Some(tz_offset),
        },
        attendees: Some(attendees),
        organizer: None,
        recurrence: None,
        recurring_event_id: None,
        original_start_time: None,
        hangout_link: None,
        reminders: None,
        account_email: Some(account_email.clone()),
        calendar_id: Some("primary".to_string()),
    };

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let client = crate::api::gcal::GcalClient::new(client_id, client_secret);
        match client
            .create_event_with_meet(&account_email, "primary", &event)
            .await
        {
            Ok(created) => {
                tx_clone.send(AppEvent::MeetingCreated(created)).ok();
            }
            Err(e) => {
                tx_clone
                    .send(AppEvent::FreeBusyError(format!("Failed to create: {}", e)))
                    .ok();
            }
        }
    });
}
