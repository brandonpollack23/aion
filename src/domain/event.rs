use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimeObject {
    pub date: Option<String>,      // YYYY-MM-DD for all-day
    pub date_time: Option<String>, // ISO 8601 for timed
    pub time_zone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Attendee {
    pub email: String,
    pub display_name: Option<String>,
    pub response_status: Option<ResponseStatus>,
    pub organizer: Option<bool>,
    #[serde(rename = "self")]
    pub is_self: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Organizer {
    pub email: String,
    pub display_name: Option<String>,
    #[serde(rename = "self")]
    pub is_self: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub method: ReminderMethod,
    pub minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reminders {
    pub use_default: Option<bool>,
    pub overrides: Option<Vec<Reminder>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResponseStatus {
    NeedsAction,
    Declined,
    Tentative,
    Accepted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

impl Default for EventStatus {
    fn default() -> Self {
        EventStatus::Confirmed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventType {
    Default,
    OutOfOffice,
    FocusTime,
    Birthday,
    WorkingLocation,
}

impl Default for EventType {
    fn default() -> Self {
        EventType::Default
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReminderMethod {
    Email,
    Popup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalEvent {
    pub id: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub html_link: Option<String>,
    pub status: EventStatus,
    pub event_type: Option<EventType>,
    pub start: TimeObject,
    pub end: TimeObject,
    pub attendees: Option<Vec<Attendee>>,
    pub organizer: Option<Organizer>,
    pub recurrence: Option<Vec<String>>,
    pub recurring_event_id: Option<String>,
    pub original_start_time: Option<TimeObject>,
    pub hangout_link: Option<String>,
    pub reminders: Option<Reminders>,
    pub account_email: Option<String>,
    pub calendar_id: Option<String>,
}

impl CalEvent {
    pub fn is_all_day(&self) -> bool {
        self.start.date.is_some() && self.start.date_time.is_none()
    }

    pub fn is_recurring(&self) -> bool {
        self.recurrence.as_ref().map_or(false, |r| !r.is_empty())
            || self.recurring_event_id.is_some()
    }

    pub fn display_title(&self) -> &str {
        let t = self.summary.trim();
        if t.is_empty() {
            "(No title)"
        } else {
            t
        }
    }

    pub fn calendar_key(&self) -> String {
        format!(
            "{}:{}",
            self.account_email.as_deref().unwrap_or(""),
            self.calendar_id.as_deref().unwrap_or("")
        )
    }

    pub fn self_response_status(&self) -> Option<&ResponseStatus> {
        self.attendees
            .as_ref()?
            .iter()
            .find(|a| a.is_self == Some(true))
            .and_then(|a| a.response_status.as_ref())
    }

    pub fn self_attendee_email(&self) -> Option<&str> {
        self.attendees
            .as_ref()?
            .iter()
            .find(|a| a.is_self == Some(true))
            .map(|a| a.email.as_str())
    }

    pub fn has_other_attendees(&self) -> bool {
        self.attendees.as_ref().map_or(false, |a| {
            a.iter()
                .any(|x| x.is_self != Some(true) && x.organizer != Some(true))
        })
    }

    pub fn parse_recurrence_rule(&self) -> Option<String> {
        let recurrence = self.recurrence.as_ref()?;
        let rrule = recurrence.iter().find(|r| r.starts_with("RRULE:"))?;
        let rule = rrule.trim_start_matches("RRULE:");
        let parts: std::collections::HashMap<&str, &str> = rule
            .split(';')
            .filter_map(|p| {
                let mut kv = p.splitn(2, '=');
                Some((kv.next()?, kv.next()?))
            })
            .collect();

        let freq = parts.get("FREQ").copied()?.to_lowercase();
        let interval: u32 = parts
            .get("INTERVAL")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let byday = parts.get("BYDAY").copied();
        let bymonthday = parts.get("BYMONTHDAY").copied();
        let count: Option<u32> = parts.get("COUNT").and_then(|v| v.parse().ok());
        let until = parts.get("UNTIL").copied();

        let mut result = if interval == 1 {
            match freq.as_str() {
                "daily" => "Daily".to_string(),
                "weekly" => "Weekly".to_string(),
                "monthly" => "Monthly".to_string(),
                "yearly" => "Yearly".to_string(),
                other => other.to_string(),
            }
        } else {
            match freq.as_str() {
                "daily" => format!("Every {} days", interval),
                "weekly" => format!("Every {} weeks", interval),
                "monthly" => format!("Every {} months", interval),
                "yearly" => format!("Every {} years", interval),
                other => format!("Every {} {}", interval, other),
            }
        };

        if let (Some(days), "weekly") = (byday, freq.as_str()) {
            let day_map = [
                ("MO", "Mon"),
                ("TU", "Tue"),
                ("WE", "Wed"),
                ("TH", "Thu"),
                ("FR", "Fri"),
                ("SA", "Sat"),
                ("SU", "Sun"),
            ];
            let mapped: Vec<&str> = days
                .split(',')
                .map(|d| {
                    // strip any leading ordinal digit (e.g., "1MO" → "MO")
                    let d = d.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-');
                    day_map
                        .iter()
                        .find(|(k, _)| *k == d)
                        .map(|(_, v)| *v)
                        .unwrap_or(d)
                })
                .collect();
            result.push_str(&format!(" on {}", mapped.join(", ")));
        }

        if let (Some(mday), "monthly") = (bymonthday, freq.as_str()) {
            let day_num: i32 = mday.parse().unwrap_or(0);
            let suffix = match day_num.abs() % 10 {
                1 if day_num.abs() % 100 != 11 => "st",
                2 if day_num.abs() % 100 != 12 => "nd",
                3 if day_num.abs() % 100 != 13 => "rd",
                _ => "th",
            };
            if day_num < 0 {
                result.push_str(&format!(" on the {}{}  from end", day_num.abs(), suffix));
            } else {
                result.push_str(&format!(" on the {}{}", day_num, suffix));
            }
        }

        // Append end condition
        if let Some(n) = count {
            result.push_str(&format!(", {} time{}", n, if n == 1 { "" } else { "s" }));
        } else if let Some(until_str) = until {
            // UNTIL is YYYYMMDD[T...] — display as "until Mon DD, YYYY"
            if until_str.len() >= 8 {
                let y = &until_str[0..4];
                let m = &until_str[4..6];
                let d = &until_str[6..8];
                let month_name = match m {
                    "01" => "Jan",
                    "02" => "Feb",
                    "03" => "Mar",
                    "04" => "Apr",
                    "05" => "May",
                    "06" => "Jun",
                    "07" => "Jul",
                    "08" => "Aug",
                    "09" => "Sep",
                    "10" => "Oct",
                    "11" => "Nov",
                    "12" => "Dec",
                    _ => m,
                };
                result.push_str(&format!(
                    ", until {} {} {}",
                    month_name,
                    d.trim_start_matches('0'),
                    y
                ));
            }
        }

        Some(result)
    }
}

pub fn get_attendance_indicator(
    status: Option<&ResponseStatus>,
    has_attendees: bool,
) -> &'static str {
    match status {
        Some(ResponseStatus::Accepted) => "✓ ",
        Some(ResponseStatus::Declined) => "✗ ",
        Some(ResponseStatus::Tentative) => "? ",
        Some(ResponseStatus::NeedsAction) => "! ",
        None if has_attendees => "! ",
        None => "",
    }
}
