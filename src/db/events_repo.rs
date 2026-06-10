use anyhow::Result;
use rusqlite::{Connection, Row};

use crate::domain::event::{
    Attendee, CalEvent, EventStatus, EventType, Organizer, Reminders, TimeObject,
};

fn row_to_event(row: &Row<'_>) -> rusqlite::Result<CalEvent> {
    let status_str: String = row.get("status")?;
    let status = match status_str.as_str() {
        "tentative" => EventStatus::Tentative,
        "cancelled" => EventStatus::Cancelled,
        _ => EventStatus::Confirmed,
    };

    let event_type_str: Option<String> = row.get("event_type")?;
    let event_type = event_type_str.as_deref().map(|s| match s {
        "outOfOffice" => EventType::OutOfOffice,
        "focusTime" => EventType::FocusTime,
        "birthday" => EventType::Birthday,
        "workingLocation" => EventType::WorkingLocation,
        _ => EventType::Default,
    });

    let start = TimeObject {
        date: row.get("start_date")?,
        date_time: row.get("start_date_time")?,
        time_zone: row.get("start_time_zone")?,
    };
    let end = TimeObject {
        date: row.get("end_date")?,
        date_time: row.get("end_date_time")?,
        time_zone: row.get("end_time_zone")?,
    };

    let recurrence: Option<Vec<String>> = row
        .get::<_, Option<String>>("recurrence_json")?
        .and_then(|s| serde_json::from_str(&s).ok());

    let original_start: Option<TimeObject> = row
        .get::<_, Option<String>>("original_start_json")?
        .and_then(|s| serde_json::from_str(&s).ok());

    let attendees: Option<Vec<Attendee>> = row
        .get::<_, Option<String>>("attendees_json")?
        .and_then(|s| serde_json::from_str(&s).ok());

    let organizer: Option<Organizer> = row
        .get::<_, Option<String>>("organizer_json")?
        .and_then(|s| serde_json::from_str(&s).ok());

    let reminders: Option<Reminders> = row
        .get::<_, Option<String>>("reminders_json")?
        .and_then(|s| serde_json::from_str(&s).ok());

    Ok(CalEvent {
        id: row.get("id")?,
        summary: row.get("summary")?,
        description: row.get("description")?,
        location: row.get("location")?,
        html_link: row.get("html_link")?,
        status,
        event_type,
        start,
        end,
        attendees,
        organizer,
        recurrence,
        recurring_event_id: row.get("recurring_event_id")?,
        original_start_time: original_start,
        hangout_link: row.get("hangout_link")?,
        reminders,
        account_email: row.get("account_email")?,
        calendar_id: row.get("calendar_id")?,
    })
}

pub fn get_all_events(conn: &Connection) -> Result<Vec<CalEvent>> {
    let mut stmt = conn.prepare(
        "SELECT id, summary, description, location, html_link, status, event_type, visibility,
                start_date, start_date_time, start_time_zone,
                end_date, end_date_time, end_time_zone,
                recurrence_json, recurring_event_id, original_start_json,
                attendees_json, organizer_json, hangout_link, reminders_json,
                account_email, calendar_id, created_at, updated_at
         FROM events
         ORDER BY COALESCE(start_date_time, start_date) ASC",
    )?;

    let events = stmt
        .query_map([], row_to_event)?
        .filter_map(|r| r.ok())
        .collect();

    Ok(events)
}

pub fn upsert_event(conn: &Connection, event: &CalEvent) -> Result<()> {
    let recurrence_json = event
        .recurrence
        .as_ref()
        .and_then(|r| serde_json::to_string(r).ok());
    let original_start_json = event
        .original_start_time
        .as_ref()
        .and_then(|o| serde_json::to_string(o).ok());
    let attendees_json = event
        .attendees
        .as_ref()
        .and_then(|a| serde_json::to_string(a).ok());
    let organizer_json = event
        .organizer
        .as_ref()
        .and_then(|o| serde_json::to_string(o).ok());
    let reminders_json = event
        .reminders
        .as_ref()
        .and_then(|r| serde_json::to_string(r).ok());

    let status_str = match event.status {
        EventStatus::Confirmed => "confirmed",
        EventStatus::Tentative => "tentative",
        EventStatus::Cancelled => "cancelled",
    };
    let event_type_str = event.event_type.as_ref().map(|t| match t {
        EventType::OutOfOffice => "outOfOffice",
        EventType::FocusTime => "focusTime",
        EventType::Birthday => "birthday",
        EventType::WorkingLocation => "workingLocation",
        EventType::Default => "default",
    });

    conn.execute(
        "INSERT OR REPLACE INTO events
             (id, summary, description, location, html_link, status, event_type,
              start_date, start_date_time, start_time_zone,
              end_date, end_date_time, end_time_zone,
              recurrence_json, recurring_event_id, original_start_json,
              attendees_json, organizer_json, hangout_link, reminders_json,
              account_email, calendar_id,
              created_at, updated_at)
         VALUES
             (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,
              datetime('now'), datetime('now'))",
        rusqlite::params![
            event.id,
            event.summary,
            event.description,
            event.location,
            event.html_link,
            status_str,
            event_type_str,
            event.start.date,
            event.start.date_time,
            event.start.time_zone,
            event.end.date,
            event.end.date_time,
            event.end.time_zone,
            recurrence_json,
            event.recurring_event_id,
            original_start_json,
            attendees_json,
            organizer_json,
            event.hangout_link,
            reminders_json,
            event.account_email,
            event.calendar_id,
        ],
    )?;
    Ok(())
}

pub fn get_events_with_reminders(conn: &Connection) -> Result<Vec<CalEvent>> {
    let mut stmt = conn.prepare(
        "SELECT id, summary, description, location, html_link, status, event_type, visibility,
                start_date, start_date_time, start_time_zone,
                end_date, end_date_time, end_time_zone,
                recurrence_json, recurring_event_id, original_start_json,
                attendees_json, organizer_json, hangout_link, reminders_json,
                account_email, calendar_id, created_at, updated_at
         FROM events
         WHERE reminders_json IS NOT NULL
         ORDER BY COALESCE(start_date_time, start_date) ASC",
    )?;

    let events = stmt
        .query_map([], row_to_event)?
        .filter_map(|r| r.ok())
        .collect();

    Ok(events)
}

pub fn delete_event(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM events WHERE id = ?1", [id])?;
    Ok(())
}

pub fn delete_events_for_account(conn: &Connection, account_email: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM events WHERE account_email = ?1",
        [account_email],
    )?;
    Ok(())
}

pub fn delete_all_events(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM events", [])?;
    Ok(())
}
