use std::collections::HashMap;

use crate::domain::event::{
    Attendee, CalEvent, EventStatus, EventType, Organizer, Reminder, ReminderMethod, Reminders,
    ResponseStatus, TimeObject,
};

// ── Line unfolding ─────────────────────────────────────────────────────────────

fn unfold_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        if raw.starts_with(' ') || raw.starts_with('\t') {
            if let Some(last) = lines.last_mut() {
                last.push_str(&raw[1..]);
            }
        } else {
            lines.push(raw.to_string());
        }
    }
    lines
}

// ── Property parsing ───────────────────────────────────────────────────────────

struct ICalProp {
    name: String,
    params: HashMap<String, String>,
    value: String,
}

fn parse_property(line: &str) -> ICalProp {
    let bytes = line.as_bytes();
    let mut colon = None;
    let mut in_quotes = false;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'"' => in_quotes = !in_quotes,
            b':' if !in_quotes => {
                colon = Some(i);
                break;
            }
            _ => {}
        }
    }
    let (name_and_params, value) = match colon {
        Some(i) => (&line[..i], &line[i + 1..]),
        None => (line, ""),
    };

    let mut parts = name_and_params.split(';');
    let name = parts.next().unwrap_or("").to_uppercase();
    let mut params = HashMap::new();
    for part in parts {
        if let Some(eq) = part.find('=') {
            let k = part[..eq].to_uppercase();
            let v = part[eq + 1..].trim_matches('"').to_string();
            params.insert(k, v);
        }
    }
    ICalProp { name, params, value: value.to_string() }
}

fn unescape_text(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

// ── Date/time conversion ───────────────────────────────────────────────────────

fn ical_date_to_time_object(value: &str, tzid: Option<&str>) -> TimeObject {
    // All-day: YYYYMMDD
    if value.len() == 8 && !value.contains('T') {
        let y = &value[..4];
        let m = &value[4..6];
        let d = &value[6..8];
        return TimeObject {
            date: Some(format!("{}-{}-{}", y, m, d)),
            date_time: None,
            time_zone: None,
        };
    }

    // Date-time: YYYYMMDDTHHMMSS[Z]
    let is_utc = value.ends_with('Z');
    let clean = value.trim_end_matches('Z');
    if clean.len() >= 15 {
        let year = &clean[..4];
        let mon = &clean[4..6];
        let day = &clean[6..8];
        let hour = &clean[9..11];
        let min = &clean[11..13];
        let sec = &clean[13..15];
        let dt = format!("{}-{}-{}T{}:{}:{}", year, mon, day, hour, min, sec);
        let tz = if is_utc {
            Some("UTC".to_string())
        } else {
            tzid.map(|s| s.to_string())
        };
        let dt_full = if is_utc { format!("{}Z", dt) } else { dt };
        return TimeObject { date: None, date_time: Some(dt_full), time_zone: tz };
    }

    // Fallback
    TimeObject { date: Some(value.to_string()), date_time: None, time_zone: None }
}

fn ical_timestamp_to_iso(value: &str) -> String {
    let t = ical_date_to_time_object(value, None);
    t.date_time.or(t.date).unwrap_or_else(|| value.to_string())
}

fn parse_duration_ms(value: &str) -> i64 {
    let s = value.trim_start_matches(['+', '-']);
    // Weeks: PnW
    if let Some(cap) = s.strip_prefix('P').and_then(|s| {
        let w = s.trim_end_matches('W').parse::<i64>().ok();
        if s.ends_with('W') { w } else { None }
    }) {
        return cap * 7 * 24 * 3600 * 1000;
    }
    let mut ms = 0i64;
    let body = s.strip_prefix('P').unwrap_or(s);
    // Days before T
    let (day_part, time_part) = if let Some(t) = body.find('T') {
        (&body[..t], &body[t + 1..])
    } else {
        (body, "")
    };
    if let Some(d) = parse_component(day_part, 'D') {
        ms += d * 24 * 3600 * 1000;
    }
    if let Some(h) = parse_component(time_part, 'H') {
        ms += h * 3600 * 1000;
    }
    if let Some(m) = parse_component(time_part, 'M') {
        ms += m * 60 * 1000;
    }
    if let Some(sec) = parse_component(time_part, 'S') {
        ms += sec * 1000;
    }
    ms
}

fn parse_component(s: &str, unit: char) -> Option<i64> {
    let idx = s.find(unit)?;
    let start = s[..idx].rfind(|c: char| !c.is_ascii_digit()).map_or(0, |i| i + 1);
    s[start..idx].parse().ok()
}

fn apply_duration(start: &TimeObject, duration_value: &str) -> TimeObject {
    let ms = parse_duration_ms(duration_value);
    if let Some(ref dt_str) = start.date_time {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(dt_str) {
            let end = dt + chrono::Duration::milliseconds(ms);
            return TimeObject {
                date: None,
                date_time: Some(end.to_rfc3339()),
                time_zone: start.time_zone.clone(),
            };
        }
        // For non-UTC datetime strings, parse more loosely
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%dT%H:%M:%S") {
            let end = dt + chrono::Duration::milliseconds(ms);
            return TimeObject {
                date: None,
                date_time: Some(end.format("%Y-%m-%dT%H:%M:%S").to_string()),
                time_zone: start.time_zone.clone(),
            };
        }
    }
    if let Some(ref date_str) = start.date {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            let days = ms / (24 * 3600 * 1000);
            let end = d + chrono::Duration::days(days);
            return TimeObject {
                date: Some(end.format("%Y-%m-%d").to_string()),
                date_time: None,
                time_zone: None,
            };
        }
    }
    start.clone()
}

// ── VEVENT parsing ─────────────────────────────────────────────────────────────

fn parse_valarm_blocks(lines: &[String]) -> (Vec<String>, Vec<Vec<String>>) {
    let mut vevent_lines: Vec<String> = Vec::new();
    let mut alarm_blocks: Vec<Vec<String>> = Vec::new();
    let mut in_alarm = false;
    let mut cur_alarm: Vec<String> = Vec::new();
    for line in lines {
        match line.as_str() {
            "BEGIN:VALARM" => {
                in_alarm = true;
            }
            "END:VALARM" => {
                in_alarm = false;
                alarm_blocks.push(cur_alarm.clone());
                cur_alarm.clear();
            }
            _ if in_alarm => cur_alarm.push(line.clone()),
            _ => vevent_lines.push(line.clone()),
        }
    }
    (vevent_lines, alarm_blocks)
}

fn parse_valarm_reminders(alarm_blocks: &[Vec<String>]) -> Option<Reminders> {
    let reminders: Vec<Reminder> = alarm_blocks
        .iter()
        .filter_map(|block| {
            let mut trigger: Option<String> = None;
            let mut action: Option<String> = None;
            for line in block {
                let prop = parse_property(line);
                match prop.name.as_str() {
                    "TRIGGER" => trigger = Some(prop.value.clone()),
                    "ACTION" => action = Some(prop.value.to_uppercase()),
                    _ => {}
                }
            }
            if action.as_deref() == Some("EMAIL") {
                return None;
            }
            let trigger_str = trigger?;
            // Only handle negative (before-event) relative triggers
            if !trigger_str.starts_with('-') {
                return None;
            }
            let ms = parse_duration_ms(&trigger_str);
            let minutes = ms / 60_000;
            if minutes == 0 {
                return None;
            }
            Some(Reminder { method: ReminderMethod::Popup, minutes })
        })
        .collect();

    if reminders.is_empty() {
        None
    } else {
        Some(Reminders { use_default: Some(false), overrides: Some(reminders) })
    }
}

fn parse_vevent(
    lines: &[String],
    account_email: Option<&str>,
    calendar_id: Option<&str>,
) -> Option<CalEvent> {
    let (vevent_lines, alarm_blocks) = parse_valarm_blocks(lines);
    let lines = &vevent_lines;

    let mut props: HashMap<String, Vec<ICalProp>> = HashMap::new();
    for line in lines {
        if line == "BEGIN:VEVENT" || line == "END:VEVENT" {
            continue;
        }
        let prop = parse_property(line);
        props.entry(prop.name.clone()).or_default().push(prop);
    }

    let uid = props.get("UID")?.first()?.value.clone();
    if uid.is_empty() {
        return None;
    }

    let composite_id = make_composite_id(account_email, calendar_id, &uid);

    let dtstart = props.get("DTSTART")?.first()?;
    let start = ical_date_to_time_object(&dtstart.value, dtstart.params.get("TZID").map(|s| s.as_str()));

    let end = if let Some(dtend) = props.get("DTEND").and_then(|v| v.first()) {
        ical_date_to_time_object(&dtend.value, dtend.params.get("TZID").map(|s| s.as_str()))
    } else if let Some(dur) = props.get("DURATION").and_then(|v| v.first()) {
        apply_duration(&start, &dur.value)
    } else {
        start.clone()
    };

    let attendees: Vec<Attendee> = props
        .get("ATTENDEE")
        .map(|v| {
            v.iter()
                .map(|a| Attendee {
                    email: a.value.trim_start_matches("mailto:").trim_start_matches("MAILTO:").to_string(),
                    display_name: a.params.get("CN").cloned(),
                    response_status: partstat_to_response_status(a.params.get("PARTSTAT").map(|s| s.as_str())),
                    organizer: if a.params.get("ROLE").map(|s| s.as_str()) == Some("CHAIR") {
                        Some(true)
                    } else {
                        None
                    },
                    is_self: None,
                })
                .collect()
        })
        .unwrap_or_default();

    let organizer = props.get("ORGANIZER").and_then(|v| v.first()).map(|o| Organizer {
        email: o.value.trim_start_matches("mailto:").trim_start_matches("MAILTO:").to_string(),
        display_name: o.params.get("CN").cloned(),
        is_self: None,
    });

    let recurrence: Vec<String> = props
        .get("RRULE")
        .map(|v| v.iter().map(|r| format!("RRULE:{}", r.value)).collect())
        .unwrap_or_default();

    let status = ical_status_to_event_status(
        props.get("STATUS").and_then(|v| v.first()).map(|p| p.value.as_str()),
    );

    let created = props
        .get("CREATED")
        .and_then(|v| v.first())
        .map(|p| ical_timestamp_to_iso(&p.value));
    let updated = props
        .get("LAST-MODIFIED")
        .and_then(|v| v.first())
        .map(|p| ical_timestamp_to_iso(&p.value));

    let summary = props
        .get("SUMMARY")
        .and_then(|v| v.first())
        .map(|p| unescape_text(&p.value))
        .unwrap_or_default();

    Some(CalEvent {
        id: composite_id,
        summary,
        description: props
            .get("DESCRIPTION")
            .and_then(|v| v.first())
            .map(|p| unescape_text(&p.value)),
        location: props
            .get("LOCATION")
            .and_then(|v| v.first())
            .map(|p| unescape_text(&p.value)),
        html_link: None,
        status,
        event_type: Some(EventType::Default),
        start,
        end,
        attendees: if attendees.is_empty() { None } else { Some(attendees) },
        organizer,
        recurrence: if recurrence.is_empty() { None } else { Some(recurrence) },
        recurring_event_id: props
            .get("RECURRENCE-ID")
            .and_then(|v| v.first())
            .map(|p| p.value.clone()),
        original_start_time: None,
        hangout_link: None,
        reminders: parse_valarm_reminders(&alarm_blocks),
        account_email: account_email.map(|s| s.to_string()),
        calendar_id: calendar_id.map(|s| s.to_string()),
    })
}

fn partstat_to_response_status(s: Option<&str>) -> Option<ResponseStatus> {
    match s.map(|v| v.to_uppercase()).as_deref() {
        Some("ACCEPTED") => Some(ResponseStatus::Accepted),
        Some("DECLINED") => Some(ResponseStatus::Declined),
        Some("TENTATIVE") => Some(ResponseStatus::Tentative),
        Some("NEEDS-ACTION") => Some(ResponseStatus::NeedsAction),
        _ => None,
    }
}

fn response_status_to_partstat(s: Option<&ResponseStatus>) -> &'static str {
    match s {
        Some(ResponseStatus::Accepted) => "ACCEPTED",
        Some(ResponseStatus::Declined) => "DECLINED",
        Some(ResponseStatus::Tentative) => "TENTATIVE",
        _ => "NEEDS-ACTION",
    }
}

fn ical_status_to_event_status(s: Option<&str>) -> EventStatus {
    match s.map(|v| v.to_uppercase()).as_deref() {
        Some("CONFIRMED") => EventStatus::Confirmed,
        Some("TENTATIVE") => EventStatus::Tentative,
        Some("CANCELLED") => EventStatus::Cancelled,
        _ => EventStatus::Confirmed,
    }
}

// ── Public composite ID helper ─────────────────────────────────────────────────

pub fn make_composite_id(account_email: Option<&str>, calendar_id: Option<&str>, uid: &str) -> String {
    format!(
        "{}:{}:{}",
        account_email.unwrap_or(""),
        calendar_id.unwrap_or(""),
        uid
    )
}

pub fn extract_uid_from_composite(
    composite_id: &str,
    account_email: &str,
    calendar_id: &str,
) -> String {
    let prefix = format!("{}:{}:", account_email, calendar_id);
    if let Some(rest) = composite_id.strip_prefix(&prefix) {
        return rest.to_string();
    }
    // Fallback: last segment after last colon
    composite_id.rsplitn(2, ':').next().unwrap_or(composite_id).to_string()
}

// ── Public API ─────────────────────────────────────────────────────────────────

pub fn parse_icalendar(
    ical_data: &str,
    account_email: Option<&str>,
    calendar_id: Option<&str>,
) -> Vec<CalEvent> {
    let lines = unfold_lines(ical_data);
    let mut events = Vec::new();
    let mut in_vevent = false;
    let mut event_lines: Vec<String> = Vec::new();

    for line in &lines {
        if line == "BEGIN:VEVENT" {
            in_vevent = true;
            event_lines = vec![line.clone()];
        } else if line == "END:VEVENT" {
            event_lines.push(line.clone());
            in_vevent = false;
            if let Some(ev) = parse_vevent(&event_lines, account_email, calendar_id) {
                events.push(ev);
            }
        } else if in_vevent {
            event_lines.push(line.clone());
        }
    }
    events
}

pub fn extract_uid(ical_data: &str) -> Option<String> {
    for line in unfold_lines(ical_data) {
        if let Some(rest) = line.strip_prefix("UID:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

// ── iCalendar generation ───────────────────────────────────────────────────────

fn escape_text(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

fn iso_to_ical_date(to: &TimeObject) -> (String, String) {
    // returns (params, value)
    if let Some(ref date) = to.date {
        if to.date_time.is_none() {
            let cleaned = date.replace('-', "");
            return (";VALUE=DATE".to_string(), cleaned);
        }
    }
    if let Some(ref dt) = to.date_time {
        let is_utc = dt.ends_with('Z') || to.time_zone.as_deref() == Some("UTC");
        // Strip to just YYYYMMDDTHHMMSS
        let stripped: String = dt
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == 'T' || *c == 'Z')
            .collect();
        let value_base = if stripped.len() >= 15 {
            format!(
                "{}{}{}T{}{}{}",
                &stripped[..4],
                &stripped[4..6],
                &stripped[6..8],
                &stripped[9..11],
                &stripped[11..13],
                &stripped[13..15]
            )
        } else {
            stripped.trim_end_matches('Z').to_string()
        };
        if is_utc {
            return ("".to_string(), format!("{}Z", value_base));
        }
        if let Some(ref tz) = to.time_zone {
            return (format!(";TZID={}", tz), value_base);
        }
        return ("".to_string(), value_base);
    }
    ("".to_string(), String::new())
}

fn fold_line(line: &str) -> String {
    if line.len() <= 75 {
        return line.to_string();
    }
    let mut result = String::new();
    result.push_str(&line[..75]);
    let mut remaining = &line[75..];
    while !remaining.is_empty() {
        result.push_str("\r\n ");
        let take = remaining.len().min(74);
        result.push_str(&remaining[..take]);
        remaining = &remaining[take..];
    }
    result
}

pub fn generate_icalendar(event: &CalEvent, uid: Option<&str>) -> String {
    let event_uid = uid
        .map(|s| s.to_string())
        .or_else(|| Some(uuid::Uuid::new_v4().to_string()))
        .unwrap();

    let now = chrono::Utc::now();
    let stamp = now.format("%Y%m%dT%H%M%SZ").to_string();

    let mut lines: Vec<String> = Vec::new();
    lines.push("BEGIN:VCALENDAR".into());
    lines.push("VERSION:2.0".into());
    lines.push("PRODID:-//Aion//Calendar//EN".into());
    lines.push("BEGIN:VEVENT".into());
    lines.push(format!("UID:{}", event_uid));
    lines.push(format!("DTSTAMP:{}", stamp));

    let (start_params, start_val) = iso_to_ical_date(&event.start);
    if !start_val.is_empty() {
        lines.push(format!("DTSTART{}:{}", start_params, start_val));
    }
    let (end_params, end_val) = iso_to_ical_date(&event.end);
    if !end_val.is_empty() {
        lines.push(format!("DTEND{}:{}", end_params, end_val));
    }

    if !event.summary.is_empty() {
        lines.push(format!("SUMMARY:{}", escape_text(&event.summary)));
    }
    if let Some(ref desc) = event.description {
        lines.push(format!("DESCRIPTION:{}", escape_text(desc)));
    }
    if let Some(ref loc) = event.location {
        lines.push(format!("LOCATION:{}", escape_text(loc)));
    }

    let status_str = match event.status {
        EventStatus::Confirmed => "CONFIRMED",
        EventStatus::Tentative => "TENTATIVE",
        EventStatus::Cancelled => "CANCELLED",
    };
    lines.push(format!("STATUS:{}", status_str));

    if let Some(ref org) = event.organizer {
        let cn = org
            .display_name
            .as_deref()
            .map(|n| format!(";CN={}", n))
            .unwrap_or_default();
        lines.push(format!("ORGANIZER{}:mailto:{}", cn, org.email));
    }

    if let Some(ref attendees) = event.attendees {
        for att in attendees {
            let mut params = Vec::new();
            if let Some(ref name) = att.display_name {
                params.push(format!("CN={}", name));
            }
            params.push(format!(
                "PARTSTAT={}",
                response_status_to_partstat(att.response_status.as_ref())
            ));
            let param_str = if params.is_empty() {
                String::new()
            } else {
                format!(";{}", params.join(";"))
            };
            lines.push(format!("ATTENDEE{}:mailto:{}", param_str, att.email));
        }
    }

    if let Some(ref recurrence) = event.recurrence {
        for rule in recurrence {
            lines.push(rule.clone());
        }
    }

    lines.push("END:VEVENT".into());
    lines.push("END:VCALENDAR".into());

    let folded: Vec<String> = lines.iter().map(|l| fold_line(l)).collect();
    folded.join("\r\n") + "\r\n"
}
