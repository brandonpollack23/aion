use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, EventStream};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use crate::auth::tokens::AccountInfo;
use crate::config::loader::load_config;
use crate::db::{connection::open_connection, events_repo::{delete_event as db_delete_event, get_all_events, upsert_event}};
use crate::keybinds::registry::handle_key;
use crate::state::app_state::AppState;
use crate::state::message::Message;
use crate::sync::background::{start_background_sync, trigger_sync_once, SyncResult};
use crate::ui::app_view::render_app;

pub enum AppEvent {
    Key(crossterm::event::KeyEvent),
    Resize(u16, u16),
    Tick,
    ShowMessage(Message),
    // Phase 3 auth
    AuthUrlReady(String),
    AuthSuccess(AccountInfo),
    AuthError(String),
    // Phase 3 sync
    SyncComplete(SyncResult),
    SyncError(String),
    TriggerSync,
    // Phase 4 CalDAV
    CalDavTestResult(Result<String, String>),
    // Phase 6: fast animation tick (80ms, only during splash)
    AnimTick,
    // Phase 5 RSVP
    RsvpComplete { event_id: String },
    RsvpError(String),
    // Phase 5 MeetWith free/busy + meeting creation
    FreeBusyLoaded(Vec<crate::domain::free_slots::TimeSlot>),
    FreeBusyError(String),
    MeetingCreated(crate::domain::event::CalEvent),
    Quit,
}

pub async fn run() -> Result<()> {
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture,
    )?;

    let result = run_app(&mut terminal).await;

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    let config = load_config();
    let client_id = config.google.client_id.clone();
    let client_secret = config.google.client_secret.clone();
    let notifications_enabled = config.notifications.enabled;
    let mut app = AppState::new(config);

    // Open DB
    let db = match open_connection() {
        Ok(conn) => Arc::new(Mutex::new(conn)),
        Err(e) => {
            tracing::warn!("Failed to open database: {}", e);
            Arc::new(Mutex::new(rusqlite::Connection::open_in_memory()?))
        }
    };

    // Load events from DB
    {
        let conn = db.lock().unwrap();
        match get_all_events(&conn) {
            Ok(events) => {
                let count = events.len();
                app.load_events(events);
                if count > 0 {
                    app.is_logged_in = true;
                }
            }
            Err(e) => tracing::warn!("Failed to load events: {}", e),
        }
    }

    // Load accounts from disk
    app.reload_accounts();

    let size = terminal.size()?;
    app.terminal_width = size.width;
    app.terminal_height = size.height;

    // Start reminder checker for desktop notifications
    if notifications_enabled {
        crate::reminders::start_reminder_checker(Arc::clone(&db));
    }

    // Start background sync if credentials are configured and accounts exist
    if let (Some(id), Some(secret)) = (client_id.clone(), client_secret.clone()) {
        if !app.accounts.is_empty() {
            app.is_syncing = true;
            start_background_sync(tx.clone(), id, secret);
        }
    }

    // Spawn terminal event reader
    let tx_term = tx.clone();
    tokio::spawn(async move {
        let mut reader = EventStream::new();
        loop {
            match reader.next().await {
                Some(Ok(Event::Key(k))) => {
                    if tx_term.send(AppEvent::Key(k)).is_err() {
                        break;
                    }
                }
                Some(Ok(Event::Resize(w, h))) => {
                    if tx_term.send(AppEvent::Resize(w, h)).is_err() {
                        break;
                    }
                }
                None => break,
                _ => {}
            }
        }
    });

    // 1-second tick
    let tx_tick = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            if tx_tick.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    // 80ms animation tick (only meaningful while splash is shown)
    let tx_anim = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(80));
        loop {
            interval.tick().await;
            if tx_anim.send(AppEvent::AnimTick).is_err() {
                break;
            }
        }
    });

    terminal.draw(|f| render_app(&app, f))?;

    loop {
        match rx.recv().await {
            Some(AppEvent::Key(key)) => {
                let should_quit = handle_key(&mut app, key, &tx).await;
                if should_quit {
                    break;
                }
            }
            Some(AppEvent::Resize(w, h)) => {
                app.terminal_width = w;
                app.terminal_height = h;
            }
            Some(AppEvent::Tick) => {
                if let Some(ref msg) = app.message {
                    if msg.created_at.elapsed() > Duration::from_secs(5) {
                        app.message = None;
                    }
                }
            }
            Some(AppEvent::ShowMessage(msg)) => {
                app.show_message(msg);
            }

            // ── Auth events ────────────────────────────────────────────────
            Some(AppEvent::AuthUrlReady(url)) => {
                app.show_message(Message::info(format!(
                    "Opening browser… if it doesn't open, visit: {}",
                    url
                )));
            }
            Some(AppEvent::AuthSuccess(account)) => {
                app.is_auth_loading = false;
                app.reload_accounts();
                app.show_message(Message::success(format!(
                    "Logged in as {}",
                    account.email
                )));
                // Trigger immediate sync for the new account
                if let (Some(id), Some(secret)) = (client_id.clone(), client_secret.clone()) {
                    app.is_syncing = true;
                    trigger_sync_once(tx.clone(), id, secret);
                }
            }
            Some(AppEvent::AuthError(msg)) => {
                app.is_auth_loading = false;
                app.show_message(Message::error(format!("Auth failed: {}", msg)));
            }

            // ── Sync events ────────────────────────────────────────────────
            Some(AppEvent::SyncComplete(result)) => {
                app.is_syncing = false;
                app.show_splash = false;

                // Update in-memory events
                for event in &result.changed {
                    app.events.insert(event.id.clone(), event.clone());
                }
                for id in &result.deleted {
                    app.events.remove(id);
                }

                // Rebuild calendar list first, then overlay display names / colors from API
                app.rebuild_calendars_pub();
                if !result.calendars.is_empty() {
                    app.apply_api_calendars(&result.calendars);
                }
                app.auto_select_event();

                if !result.changed.is_empty() || !result.deleted.is_empty() {
                    app.is_logged_in = true;
                }

                // Persist to DB in background
                let db_clone = Arc::clone(&db);
                let changed_for_db = result.changed.clone();
                let deleted_for_db = result.deleted.clone();
                let new_tokens = result.sync_tokens.clone();
                tokio::task::spawn_blocking(move || {
                    // Update sync tokens
                    if !new_tokens.is_empty() {
                        let mut store = crate::db::sync_tokens::load_sync_tokens();
                        for (k, v) in &new_tokens {
                            store.tokens.insert(k.clone(), v.clone());
                        }
                        let _ = crate::db::sync_tokens::save_sync_tokens(&store);
                    }
                    // Upsert/delete events
                    if let Ok(conn) = db_clone.lock() {
                        for event in &changed_for_db {
                            let _ = upsert_event(&conn, event);
                        }
                        for id in &deleted_for_db {
                            let _ = db_delete_event(&conn, id);
                        }
                    }
                });
            }
            Some(AppEvent::SyncError(msg)) => {
                app.is_syncing = false;
                tracing::warn!("Sync error: {}", msg);
                // Only show persistent errors (not transient network issues)
                if msg.contains("expired") || msg.contains("revoked") || msg.contains("login") {
                    app.show_message(Message::error(msg));
                }
            }
            Some(AppEvent::TriggerSync) => {
                if let (Some(id), Some(secret)) = (client_id.clone(), client_secret.clone()) {
                    if !app.is_syncing {
                        app.is_syncing = true;
                        trigger_sync_once(tx.clone(), id, secret);
                    }
                } else {
                    app.show_message(Message::error(
                        "Google credentials not configured. Add google.clientId and google.clientSecret to config.toml",
                    ));
                }
            }

            Some(AppEvent::CalDavTestResult(result)) => {
                if let Some(ref mut cd) = app.caldav_dialog {
                    cd.testing = false;
                    cd.test_result = Some(result);
                }
            }

            // ── Phase 5: RSVP ─────────────────────────────────────────────
            Some(AppEvent::RsvpComplete { event_id }) => {
                app.show_message(Message::success("RSVP updated."));
                // Re-select the event to keep context
                app.selected_event_id = Some(event_id);
            }
            Some(AppEvent::RsvpError(msg)) => {
                app.show_message(Message::error(format!("RSVP failed: {}", msg)));
            }

            // ── Phase 5: MeetWith free/busy ────────────────────────────────
            Some(AppEvent::FreeBusyLoaded(slots)) => {
                if let Some(ref mut mw) = app.meet_with {
                    mw.slots = slots;
                    mw.slots_loading = false;
                    mw.slots_cursor = 0;
                }
            }
            Some(AppEvent::FreeBusyError(msg)) => {
                if let Some(ref mut mw) = app.meet_with {
                    mw.slots_loading = false;
                }
                app.show_message(Message::error(format!("Free/busy error: {}", msg)));
            }
            Some(AppEvent::MeetingCreated(event)) => {
                let id = event.id.clone();
                app.events.insert(id.clone(), event);
                app.rebuild_calendars_pub();
                app.meet_with = None;
                app.pop_overlay();
                app.show_message(Message::success("Meeting created successfully."));
                app.selected_event_id = Some(id);
                app.auto_select_event();
            }

            Some(AppEvent::AnimTick) => {
                if app.show_splash {
                    app.splash_tick = app.splash_tick.wrapping_add(1);
                } else {
                    // Once splash is done, skip the redraw for anim ticks to avoid noise
                    continue;
                }
            }

            Some(AppEvent::Quit) | None => break,
        }

        terminal.draw(|f| render_app(&app, f))?;
    }

    Ok(())
}
