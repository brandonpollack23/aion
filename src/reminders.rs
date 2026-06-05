use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};

use crate::db::events_repo::get_events_with_reminders;
use crate::domain::event::{CalEvent, ReminderMethod};

pub fn start_reminder_checker(db: Arc<Mutex<rusqlite::Connection>>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        let mut fired: HashSet<String> = HashSet::new();

        loop {
            interval.tick().await;

            let db_clone = Arc::clone(&db);
            let events = tokio::task::spawn_blocking(move || {
                db_clone
                    .lock()
                    .ok()
                    .and_then(|conn| get_events_with_reminders(&conn).ok())
                    .unwrap_or_default()
            })
            .await
            .unwrap_or_default();

            let now = Utc::now();

            for event in &events {
                let Some(reminders) = &event.reminders else {
                    continue;
                };
                // Skip "use calendar default" — we don't know the default minutes
                if reminders.use_default == Some(true) {
                    continue;
                }
                let Some(overrides) = &reminders.overrides else {
                    continue;
                };

                let Some(start_utc) = parse_event_start_utc(event) else {
                    continue;
                };

                for reminder in overrides {
                    if reminder.method != ReminderMethod::Popup {
                        continue;
                    }
                    let key = format!("{}:{}", event.id, reminder.minutes);
                    if fired.contains(&key) {
                        continue;
                    }

                    let trigger_time =
                        start_utc - chrono::Duration::minutes(reminder.minutes);
                    let delta = now.signed_duration_since(trigger_time);

                    // Fire if we're within a 60-second window after the trigger time
                    if delta >= chrono::Duration::zero()
                        && delta < chrono::Duration::seconds(60)
                    {
                        let body = reminder_body(&start_utc, &now);
                        crate::notifications::send(&event.summary, &body);
                        fired.insert(key);
                        tracing::info!(
                            "Reminder fired for event '{}' ({} min before)",
                            event.summary,
                            reminder.minutes
                        );
                    }
                }
            }
        }
    });
}

fn parse_event_start_utc(event: &CalEvent) -> Option<DateTime<Utc>> {
    let dt_str = event.start.date_time.as_deref()?;

    // RFC 3339 / UTC (ends with Z or has +offset)
    if let Ok(dt) = DateTime::parse_from_rfc3339(dt_str) {
        return Some(dt.with_timezone(&Utc));
    }

    // Naive datetime — apply timezone hint if available
    if let Ok(naive) = NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%dT%H:%M:%S") {
        if let Some(tz_str) = &event.start.time_zone {
            if let Ok(tz) = tz_str.parse::<chrono_tz::Tz>() {
                return tz
                    .from_local_datetime(&naive)
                    .single()
                    .map(|dt| dt.with_timezone(&Utc));
            }
        }
        // Fallback: treat as local time
        return Local
            .from_local_datetime(&naive)
            .single()
            .map(|dt| dt.with_timezone(&Utc));
    }

    None
}

fn reminder_body(start_utc: &DateTime<Utc>, now: &DateTime<Utc>) -> String {
    let mins_until = start_utc.signed_duration_since(*now).num_minutes();
    let start_local = start_utc.with_timezone(&Local);
    let time_str = start_local.format("%H:%M").to_string();
    if mins_until <= 1 {
        format!("Starting now · {}", time_str)
    } else {
        format!("In {} min · {}", mins_until, time_str)
    }
}
