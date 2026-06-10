use chrono::{
    DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike, Weekday,
};
use chrono_tz::Tz;

use crate::domain::event::{CalEvent, TimeObject};

pub fn get_local_timezone() -> String {
    iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".to_string())
}

pub fn parse_tz(tz_str: &str) -> Option<Tz> {
    tz_str.parse().ok()
}

/// Parse a TimeObject into a UTC DateTime, then convert to local.
pub fn parse_time_object(time: &TimeObject, target_tz: &str) -> Option<DateTime<Tz>> {
    let tz: Tz = parse_tz(target_tz).unwrap_or(chrono_tz::UTC);

    if let Some(ref dt_str) = time.date_time {
        // Try to parse as RFC3339 first
        if let Ok(dt) = DateTime::parse_from_rfc3339(dt_str) {
            return Some(dt.with_timezone(&tz));
        }
        // Try without offset (bare ISO 8601)
        let original_tz_str = time.time_zone.as_deref().unwrap_or(target_tz);
        let original_tz: Tz = parse_tz(original_tz_str).unwrap_or(tz);
        if let Ok(naive) = NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%dT%H:%M:%S") {
            return original_tz
                .from_local_datetime(&naive)
                .single()
                .map(|dt| dt.with_timezone(&tz));
        }
        None
    } else if let Some(ref date_str) = time.date {
        let naive = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
        let naive_dt = naive.and_hms_opt(0, 0, 0)?;
        tz.from_local_datetime(&naive_dt).single()
    } else {
        None
    }
}

pub fn get_event_start(event: &CalEvent, tz: &str) -> Option<DateTime<Tz>> {
    parse_time_object(&event.start, tz)
}

pub fn get_event_end(event: &CalEvent, tz: &str) -> Option<DateTime<Tz>> {
    parse_time_object(&event.end, tz)
}

pub fn get_minutes_from_midnight(dt: &DateTime<Tz>) -> i64 {
    dt.hour() as i64 * 60 + dt.minute() as i64
}

pub fn get_hour_bucket(dt: &DateTime<Tz>) -> i64 {
    dt.hour() as i64
}

pub fn get_duration_minutes(start: &DateTime<Tz>, end: &DateTime<Tz>) -> i64 {
    (*end - *start).num_minutes()
}

pub fn format_time(dt: &DateTime<Tz>) -> String {
    format!("{:02}:{:02}", dt.hour(), dt.minute())
}

pub fn format_hour_label(hour: u32) -> String {
    match hour {
        0 | 24 => "12 AM".to_string(),
        12 => "12 PM".to_string(),
        h if h < 12 => format!("{} AM", h),
        h => format!("{} PM", h - 12),
    }
}

pub fn format_day_short(date: NaiveDate) -> String {
    let weekday = match date.weekday() {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    };
    format!("{} {}", weekday, date.day())
}

pub fn format_day_header(date: NaiveDate) -> String {
    let weekday = match date.weekday() {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    };
    let month = match date.month() {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    };
    format!("{}, {} {}", weekday, month, date.day())
}

pub fn is_today(date: NaiveDate) -> bool {
    date == Local::now().date_naive()
}

pub fn is_weekend(date: NaiveDate) -> bool {
    matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

pub fn get_now_minutes() -> i64 {
    let now = Local::now();
    now.hour() as i64 * 60 + now.minute() as i64
}

pub fn get_days_range(center: NaiveDate, before: i64, after: i64) -> Vec<NaiveDate> {
    (-before..=after)
        .map(|i| center + Duration::days(i))
        .collect()
}

/// Check if an event falls on a given day (using date only, timezone-aware).
pub fn event_falls_on_day(event: &CalEvent, day: NaiveDate, tz: &str) -> bool {
    // All-day events: Google uses exclusive end date
    if let (Some(start_date), Some(end_date)) = (&event.start.date, &event.end.date) {
        if let (Ok(s), Ok(e)) = (
            NaiveDate::parse_from_str(start_date, "%Y-%m-%d"),
            NaiveDate::parse_from_str(end_date, "%Y-%m-%d"),
        ) {
            return s <= day && day < e;
        }
    }

    // Timed events: check for overlap using full datetimes. Comparing only
    // date parts drops ordinary same-day events because their end date equals
    // the day being rendered.
    let event_start = match get_event_start(event, tz) {
        Some(dt) => dt,
        None => return false,
    };
    let event_end = match get_event_end(event, tz) {
        Some(dt) => dt,
        None => return false,
    };
    let tz_parsed: Tz = parse_tz(tz).unwrap_or(chrono_tz::UTC);
    let day_start = tz_parsed
        .from_local_datetime(&day.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .unwrap_or_else(|| tz_parsed.from_utc_datetime(&day.and_hms_opt(0, 0, 0).unwrap()));
    let day_end = day_start + Duration::days(1);

    event_start < day_end && event_end > day_start
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::EventStatus;

    fn timed_event(start: &str, end: &str) -> CalEvent {
        CalEvent {
            id: "event".to_string(),
            summary: "event".to_string(),
            description: None,
            location: None,
            html_link: None,
            status: EventStatus::Confirmed,
            event_type: None,
            start: TimeObject {
                date: None,
                date_time: Some(start.to_string()),
                time_zone: None,
            },
            end: TimeObject {
                date: None,
                date_time: Some(end.to_string()),
                time_zone: None,
            },
            attendees: None,
            organizer: None,
            recurrence: None,
            recurring_event_id: None,
            original_start_time: None,
            hangout_link: None,
            reminders: None,
            account_email: None,
            calendar_id: None,
        }
    }

    fn all_day_event(start: &str, end: &str) -> CalEvent {
        let mut event = timed_event("", "");
        event.start = TimeObject {
            date: Some(start.to_string()),
            date_time: None,
            time_zone: None,
        };
        event.end = TimeObject {
            date: Some(end.to_string()),
            date_time: None,
            time_zone: None,
        };
        event
    }

    #[test]
    fn same_day_timed_event_falls_on_its_day() {
        let event = timed_event("2026-06-10T08:30:00+09:00", "2026-06-10T09:00:00+09:00");

        assert!(event_falls_on_day(
            &event,
            NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
            "Asia/Tokyo"
        ));
    }

    #[test]
    fn cross_midnight_timed_event_falls_on_both_days() {
        let event = timed_event("2026-06-10T23:30:00+09:00", "2026-06-11T00:30:00+09:00");

        assert!(event_falls_on_day(
            &event,
            NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
            "Asia/Tokyo"
        ));
        assert!(event_falls_on_day(
            &event,
            NaiveDate::from_ymd_opt(2026, 6, 11).unwrap(),
            "Asia/Tokyo"
        ));
    }

    #[test]
    fn timed_event_ending_at_midnight_is_only_on_previous_day() {
        let event = timed_event("2026-06-10T23:30:00+09:00", "2026-06-11T00:00:00+09:00");

        assert!(event_falls_on_day(
            &event,
            NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
            "Asia/Tokyo"
        ));
        assert!(!event_falls_on_day(
            &event,
            NaiveDate::from_ymd_opt(2026, 6, 11).unwrap(),
            "Asia/Tokyo"
        ));
    }

    #[test]
    fn all_day_event_uses_exclusive_end_date() {
        let event = all_day_event("2026-06-10", "2026-06-12");

        assert!(event_falls_on_day(
            &event,
            NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
            "Asia/Tokyo"
        ));
        assert!(event_falls_on_day(
            &event,
            NaiveDate::from_ymd_opt(2026, 6, 11).unwrap(),
            "Asia/Tokyo"
        ));
        assert!(!event_falls_on_day(
            &event,
            NaiveDate::from_ymd_opt(2026, 6, 12).unwrap(),
            "Asia/Tokyo"
        ));
    }
}
