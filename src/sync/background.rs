use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use crate::api::caldav::sync_caldav_calendar;
use crate::api::gcal::{CalendarListEntry, GcalClient};
use crate::app::AppEvent;
use crate::auth::tokens::{get_accounts, AccountType};
use crate::config::loader::load_config;
use crate::db::sync_tokens::{get_sync_token, sync_key};
use crate::domain::event::CalEvent;

#[derive(Debug)]
pub struct SyncResult {
    pub changed: Vec<CalEvent>,
    pub deleted: Vec<String>,
    pub sync_tokens: HashMap<String, String>,
    pub calendars: Vec<CalendarListEntry>,
}

/// Start background sync task — fires every 30 seconds.
pub fn start_background_sync(
    tx: UnboundedSender<AppEvent>,
    client_id: String,
    client_secret: String,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let gcal = GcalClient::new(client_id.clone(), client_secret.clone());
            match perform_sync(&gcal).await {
                Ok(result) => {
                    tx.send(AppEvent::SyncComplete(result)).ok();
                }
                Err(e) => {
                    tracing::warn!("Background sync error: {}", e);
                    tx.send(AppEvent::SyncError(e.to_string())).ok();
                }
            }
        }
    });
}

/// Trigger an immediate one-shot sync (e.g. after login).
pub fn trigger_sync_once(tx: UnboundedSender<AppEvent>, client_id: String, client_secret: String) {
    tokio::spawn(async move {
        let gcal = GcalClient::new(client_id, client_secret);
        match perform_sync(&gcal).await {
            Ok(result) => {
                tx.send(AppEvent::SyncComplete(result)).ok();
            }
            Err(e) => {
                tracing::warn!("Sync error: {}", e);
                tx.send(AppEvent::SyncError(e.to_string())).ok();
            }
        }
    });
}

async fn perform_sync(gcal: &GcalClient) -> Result<SyncResult> {
    let accounts = get_accounts();
    let google_accounts: Vec<_> = accounts
        .iter()
        .filter(|a| a.account.account_type == AccountType::Google)
        .collect();

    if google_accounts.is_empty() {
        return Ok(SyncResult {
            changed: vec![],
            deleted: vec![],
            sync_tokens: HashMap::new(),
            calendars: vec![],
        });
    }

    let now = Utc::now();
    let time_min = (now - chrono::Duration::days(90))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let time_max = (now + chrono::Duration::days(180))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let mut all_changed: Vec<CalEvent> = Vec::new();
    let mut all_deleted: Vec<String> = Vec::new();
    let mut new_sync_tokens: HashMap<String, String> = HashMap::new();
    let mut all_calendars: Vec<CalendarListEntry> = Vec::new();

    for account in &google_accounts {
        let email = &account.account.email;
        let cals = match gcal.get_calendar_list(email).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to get calendar list for {}: {}", email, e);
                continue;
            }
        };

        all_calendars.extend(cals.iter().cloned());

        for cal in &cals {
            let key = sync_key(email, &cal.id);
            let existing_token = get_sync_token(email, &cal.id);

            match existing_token {
                Some(token) => {
                    // Incremental sync
                    match gcal.incremental_sync(email, &cal.id, &token).await {
                        Ok(result) => {
                            if result.full_sync_required {
                                // Fallback to full sync
                                match gcal
                                    .fetch_events(email, &cal.id, &time_min, &time_max)
                                    .await
                                {
                                    Ok((events, new_token)) => {
                                        tracing::info!(
                                            "Full sync (fallback) {}/{}: {} events",
                                            email,
                                            cal.id,
                                            events.len()
                                        );
                                        all_changed.extend(events);
                                        if let Some(t) = new_token {
                                            new_sync_tokens.insert(key, t);
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Full sync fallback failed for {}/{}: {}",
                                            email,
                                            cal.id,
                                            e
                                        );
                                    }
                                }
                            } else {
                                tracing::info!(
                                    "Incremental sync {}/{}: +{} changed, -{} deleted",
                                    email,
                                    cal.id,
                                    result.changed.len(),
                                    result.deleted.len()
                                );
                                all_changed.extend(result.changed);
                                all_deleted.extend(result.deleted);
                                if let Some(t) = result.next_sync_token {
                                    new_sync_tokens.insert(key, t);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Incremental sync failed for {}/{}: {}",
                                email,
                                cal.id,
                                e
                            );
                        }
                    }
                }
                None => {
                    // First sync for this calendar
                    match gcal
                        .fetch_events(email, &cal.id, &time_min, &time_max)
                        .await
                    {
                        Ok((events, new_token)) => {
                            tracing::info!(
                                "Full sync {}/{}: {} events",
                                email,
                                cal.id,
                                events.len()
                            );
                            all_changed.extend(events);
                            if let Some(t) = new_token {
                                new_sync_tokens.insert(key, t);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Full sync failed for {}/{}: {}", email, cal.id, e);
                        }
                    }
                }
            }
        }
    }

    // ── CalDAV accounts ────────────────────────────────────────────────────
    let config = load_config();
    for caldav_account in &config.caldav {
        let email = &caldav_account.email;

        // Get calendar list for this account
        let cals = match crate::api::caldav::get_caldav_calendars(caldav_account).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("CalDAV: failed to get calendars for {}: {}", email, e);
                continue;
            }
        };

        all_calendars.extend(cals.iter().cloned());

        for cal in &cals {
            let key = sync_key(email, &cal.id);
            let stored_ctag = get_sync_token(email, &cal.id);

            match sync_caldav_calendar(
                caldav_account,
                &cal.id,
                &time_min,
                &time_max,
                stored_ctag.as_deref(),
            )
            .await
            {
                Ok(result) => {
                    if result.full_sync {
                        all_changed.extend(result.events);
                    }
                    if let Some(ctag) = result.ctag {
                        new_sync_tokens.insert(key, ctag);
                    }
                }
                Err(e) => {
                    tracing::warn!("CalDAV sync failed for {}/{}: {}", email, cal.id, e);
                }
            }
        }
    }

    Ok(SyncResult {
        changed: all_changed,
        deleted: all_deleted,
        sync_tokens: new_sync_tokens,
        calendars: all_calendars,
    })
}
