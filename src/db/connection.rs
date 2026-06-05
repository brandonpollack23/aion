use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

pub fn db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("aion")
        .join("aion.db")
}

pub fn open_connection() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    run_migrations(&conn)?;
    Ok(conn)
}

fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            summary TEXT NOT NULL,
            description TEXT,
            location TEXT,
            html_link TEXT,
            status TEXT NOT NULL DEFAULT 'confirmed',
            event_type TEXT,
            visibility TEXT,
            start_date TEXT,
            start_date_time TEXT,
            start_time_zone TEXT,
            end_date TEXT,
            end_date_time TEXT,
            end_time_zone TEXT,
            recurrence_json TEXT,
            recurring_event_id TEXT,
            original_start_json TEXT,
            attendees_json TEXT,
            organizer_json TEXT,
            hangout_link TEXT,
            reminders_json TEXT,
            account_email TEXT,
            calendar_id TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_events_start
            ON events(COALESCE(start_date_time, start_date));
        CREATE INDEX IF NOT EXISTS idx_events_end
            ON events(COALESCE(end_date_time, end_date));
        CREATE INDEX IF NOT EXISTS idx_events_calendar
            ON events(account_email, calendar_id);
        CREATE INDEX IF NOT EXISTS idx_events_status
            ON events(status);",
    )?;
    Ok(())
}
