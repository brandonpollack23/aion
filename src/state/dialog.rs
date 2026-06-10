use chrono::{Local, NaiveDate, TimeZone};

use crate::domain::event::{CalEvent, EventStatus, EventType, TimeObject};
use crate::domain::natural_date::{format_parsed_preview, parse_natural_date};

// Field indices — used as usize in active_field
pub const F_TITLE: usize = 0;
pub const F_EVENT_TYPE: usize = 1;
pub const F_ALL_DAY: usize = 2;
pub const F_WHEN: usize = 3;
pub const F_START_DATE: usize = 4;
pub const F_START_TIME: usize = 5;
pub const F_END_DATE: usize = 6;
pub const F_END_TIME: usize = 7;
pub const F_LOCATION: usize = 8;
pub const F_NOTES: usize = 9;
pub const FIELD_COUNT: usize = 10;

pub const EVENT_TYPE_LABELS: &[&str] = &["Event", "Out of office", "Focus time"];

#[derive(Debug, Clone)]
pub struct DialogState {
    pub is_edit: bool,
    pub event_id: Option<String>,
    pub active_field: usize,

    pub title: String,
    pub event_type_idx: usize,
    pub is_all_day: bool,

    pub when_input: String,
    pub when_preview: Option<String>,

    pub start_date: String,
    pub start_time: String,
    pub end_date: String,
    pub end_time: String,

    pub location: String,
    pub notes: String,

    pub calendar_key: Option<String>,
}

impl DialogState {
    pub fn new_event(day: NaiveDate) -> Self {
        let date_str = day.format("%Y-%m-%d").to_string();
        Self {
            is_edit: false,
            event_id: None,
            active_field: F_TITLE,
            title: String::new(),
            event_type_idx: 0,
            is_all_day: false,
            when_input: String::new(),
            when_preview: None,
            start_date: date_str.clone(),
            start_time: "09:00".to_string(),
            end_date: date_str,
            end_time: "10:00".to_string(),
            location: String::new(),
            notes: String::new(),
            calendar_key: None,
        }
    }

    pub fn from_event(event: &CalEvent) -> Self {
        let (start_date, start_time, is_all_day) = if let Some(ref d) = event.start.date {
            (d.clone(), String::new(), true)
        } else if let Some(ref dt) = event.start.date_time {
            let parsed = chrono::DateTime::parse_from_rfc3339(dt)
                .map(|d| d.with_timezone(&Local))
                .ok();
            if let Some(p) = parsed {
                (
                    p.format("%Y-%m-%d").to_string(),
                    p.format("%H:%M").to_string(),
                    false,
                )
            } else {
                (String::new(), String::new(), false)
            }
        } else {
            (String::new(), String::new(), false)
        };

        let (end_date, end_time) = if let Some(ref d) = event.end.date {
            // GCal all-day end is exclusive; show inclusive (subtract 1 day)
            let adjusted = chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
                .map(|d| {
                    (d - chrono::Duration::days(1))
                        .format("%Y-%m-%d")
                        .to_string()
                })
                .unwrap_or_else(|_| d.clone());
            (adjusted, String::new())
        } else if let Some(ref dt) = event.end.date_time {
            let parsed = chrono::DateTime::parse_from_rfc3339(dt)
                .map(|d| d.with_timezone(&Local))
                .ok();
            if let Some(p) = parsed {
                (
                    p.format("%Y-%m-%d").to_string(),
                    p.format("%H:%M").to_string(),
                )
            } else {
                (String::new(), String::new())
            }
        } else {
            (String::new(), String::new())
        };

        let event_type_idx = match event.event_type {
            Some(EventType::OutOfOffice) => 1,
            Some(EventType::FocusTime) => 2,
            _ => 0,
        };

        Self {
            is_edit: true,
            event_id: Some(event.id.clone()),
            active_field: F_TITLE,
            title: event.summary.clone(),
            event_type_idx,
            is_all_day,
            when_input: String::new(),
            when_preview: None,
            start_date,
            start_time,
            end_date,
            end_time,
            location: event.location.clone().unwrap_or_default(),
            notes: event.description.clone().unwrap_or_default(),
            calendar_key: event.account_email.as_ref().map(|e| {
                format!(
                    "{}:{}",
                    e,
                    event.calendar_id.as_deref().unwrap_or("primary")
                )
            }),
        }
    }

    pub fn active_text_mut(&mut self) -> Option<&mut String> {
        match self.active_field {
            F_TITLE => Some(&mut self.title),
            F_WHEN => Some(&mut self.when_input),
            F_START_DATE => Some(&mut self.start_date),
            F_START_TIME => Some(&mut self.start_time),
            F_END_DATE => Some(&mut self.end_date),
            F_END_TIME => Some(&mut self.end_time),
            F_LOCATION => Some(&mut self.location),
            F_NOTES => Some(&mut self.notes),
            _ => None,
        }
    }

    pub fn push_char(&mut self, c: char) {
        if let Some(f) = self.active_text_mut() {
            f.push(c);
        } else if self.active_field == F_EVENT_TYPE {
            self.event_type_idx = (self.event_type_idx + 1) % EVENT_TYPE_LABELS.len();
        } else if self.active_field == F_ALL_DAY && c == ' ' {
            self.toggle_all_day();
        }
        if self.active_field == F_WHEN {
            self.refresh_when_preview();
        }
    }

    pub fn pop_char(&mut self) {
        if let Some(f) = self.active_text_mut() {
            f.pop();
        }
        if self.active_field == F_WHEN {
            self.refresh_when_preview();
        }
    }

    pub fn toggle_all_day(&mut self) {
        self.is_all_day = !self.is_all_day;
        if self.is_all_day {
            self.start_time.clear();
            self.end_time.clear();
        } else if self.start_time.is_empty() {
            self.start_time = "09:00".to_string();
            self.end_time = "10:00".to_string();
        }
    }

    pub fn next_field(&mut self) {
        let mut f = (self.active_field + 1) % FIELD_COUNT;
        if self.is_all_day {
            while f == F_START_TIME || f == F_END_TIME {
                f = (f + 1) % FIELD_COUNT;
            }
        }
        self.active_field = f;
    }

    pub fn prev_field(&mut self) {
        let mut f = if self.active_field == 0 {
            FIELD_COUNT - 1
        } else {
            self.active_field - 1
        };
        if self.is_all_day {
            while f == F_START_TIME || f == F_END_TIME {
                f = if f == 0 { FIELD_COUNT - 1 } else { f - 1 };
            }
        }
        self.active_field = f;
    }

    pub fn apply_when_input(&mut self) {
        let input = self.when_input.clone();
        if let Some(parsed) = parse_natural_date(&input) {
            self.start_date = parsed.date.format("%Y-%m-%d").to_string();
            if let Some((h, m)) = parsed.time {
                self.is_all_day = false;
                self.start_time = format!("{:02}:{:02}", h, m);
                if let Some((eh, em)) = parsed.end_time {
                    self.end_time = format!("{:02}:{:02}", eh, em);
                } else {
                    // Default 1 hour end
                    let end_h = if h + 1 < 24 { h + 1 } else { 23 };
                    self.end_time = format!("{:02}:{:02}", end_h, m);
                }
                self.end_date = parsed.end_date.map_or_else(
                    || self.start_date.clone(),
                    |d| d.format("%Y-%m-%d").to_string(),
                );
            } else {
                self.is_all_day = true;
                self.start_time.clear();
                self.end_time.clear();
                self.end_date = parsed.end_date.map_or_else(
                    || self.start_date.clone(),
                    |d| d.format("%Y-%m-%d").to_string(),
                );
            }
            self.when_input.clear();
            self.when_preview = None;
        }
    }

    pub fn refresh_when_preview(&mut self) {
        if self.when_input.is_empty() {
            self.when_preview = None;
            return;
        }
        self.when_preview = parse_natural_date(&self.when_input).map(|p| format_parsed_preview(&p));
    }

    /// Build a CalEvent from the current dialog state.
    pub fn build_event(&self) -> CalEvent {
        let id = self.event_id.clone().unwrap_or_else(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos();
            format!("local-{:x}", nanos)
        });

        let event_type = match self.event_type_idx {
            1 => Some(EventType::OutOfOffice),
            2 => Some(EventType::FocusTime),
            _ => Some(EventType::Default),
        };

        let (start, end) = if self.is_all_day {
            let start_date = self.start_date.clone();
            // End date is exclusive in GCal
            let end_date = chrono::NaiveDate::parse_from_str(&self.end_date, "%Y-%m-%d")
                .map(|d| {
                    (d + chrono::Duration::days(1))
                        .format("%Y-%m-%d")
                        .to_string()
                })
                .unwrap_or_else(|_| self.end_date.clone());
            (
                TimeObject {
                    date: Some(start_date),
                    date_time: None,
                    time_zone: None,
                },
                TimeObject {
                    date: Some(end_date),
                    date_time: None,
                    time_zone: None,
                },
            )
        } else {
            let tz_str = crate::domain::time::get_local_timezone();
            let start_dt_str = format!("{}T{}", self.start_date, self.start_time);
            let end_dt_str = format!("{}T{}", self.end_date, self.end_time);
            // Build RFC3339 strings with UTC offset
            let start_iso = chrono::NaiveDateTime::parse_from_str(&start_dt_str, "%Y-%m-%dT%H:%M")
                .map(|naive| Local.from_local_datetime(&naive).single())
                .ok()
                .flatten()
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| format!("{}:00+00:00", start_dt_str));
            let end_iso = chrono::NaiveDateTime::parse_from_str(&end_dt_str, "%Y-%m-%dT%H:%M")
                .map(|naive| Local.from_local_datetime(&naive).single())
                .ok()
                .flatten()
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| format!("{}:00+00:00", end_dt_str));
            (
                TimeObject {
                    date: None,
                    date_time: Some(start_iso),
                    time_zone: Some(tz_str.clone()),
                },
                TimeObject {
                    date: None,
                    date_time: Some(end_iso),
                    time_zone: Some(tz_str),
                },
            )
        };

        let (account_email, calendar_id) = if let Some(ref key) = self.calendar_key {
            let mut parts = key.splitn(2, ':');
            (
                parts.next().map(String::from),
                parts.next().map(String::from),
            )
        } else {
            (None, None)
        };

        CalEvent {
            id,
            summary: self.title.clone(),
            description: if self.notes.is_empty() {
                None
            } else {
                Some(self.notes.clone())
            },
            location: if self.location.is_empty() {
                None
            } else {
                Some(self.location.clone())
            },
            html_link: None,
            status: EventStatus::Confirmed,
            event_type,
            start,
            end,
            attendees: None,
            organizer: None,
            recurrence: None,
            recurring_event_id: None,
            original_start_time: None,
            hangout_link: None,
            reminders: None,
            account_email,
            calendar_id,
        }
    }
}
