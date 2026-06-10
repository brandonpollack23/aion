use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};

#[derive(Debug, Clone)]
pub struct ParsedDate {
    pub date: NaiveDate,
    pub time: Option<(u32, u32)>, // (hour 0-23, minute 0-59)
    pub end_date: Option<NaiveDate>,
    pub end_time: Option<(u32, u32)>,
    pub is_date_range: bool,
    pub duration_minutes: Option<i64>,
}

pub fn parse_natural_date(input: &str) -> Option<ParsedDate> {
    let today = Local::now().date_naive();
    let s = input.trim();
    if s.is_empty() {
        return None;
    }

    // Range patterns first: "from X until Y", "between X and Y"
    if let Some(r) = parse_range(s, today) {
        return Some(r);
    }

    // Extract duration suffix: "for N minutes/hours/days/weeks"
    let (base, dur_mins, dur_days) = extract_duration(s);

    // Shorthand: +1d, +2w, -3d
    if let Some(date) = parse_shorthand(&base, today) {
        let end_date = dur_days.map(|d| date + Duration::days(d));
        return Some(ParsedDate {
            date,
            time: None,
            end_date,
            end_time: None,
            is_date_range: dur_days.is_some(),
            duration_minutes: dur_mins,
        });
    }

    let (date, time) = parse_date_time(&base, today)?;

    let end_time = if let (Some(mins), Some((h, m))) = (dur_mins, time) {
        let total = h as i64 * 60 + m as i64 + mins;
        Some(((total / 60 % 24) as u32, (total % 60) as u32))
    } else {
        None
    };
    let end_date = dur_days.map(|d| date + Duration::days(d));
    let is_date_range = dur_days.is_some() && time.is_none();

    Some(ParsedDate {
        date,
        time,
        end_date,
        end_time,
        is_date_range,
        duration_minutes: dur_mins,
    })
}

fn parse_range(s: &str, today: NaiveDate) -> Option<ParsedDate> {
    let sl = s.to_lowercase();

    let (start_str, end_str) = if sl.starts_with("from ") || sl.starts_with("starting ") {
        let skip = if sl.starts_with("from ") { 5 } else { 9 };
        let rest = &s[skip..];
        let rest_lower = rest.to_lowercase();
        let separators = [" until ", " to ", " till "];
        let mut found = None;
        for sep in &separators {
            if let Some(pos) = rest_lower.find(sep) {
                found = Some((&rest[..pos], &rest[pos + sep.len()..]));
                break;
            }
        }
        found?
    } else if sl.starts_with("between ") {
        let rest = &s[8..];
        let rest_lower = rest.to_lowercase();
        let pos = rest_lower.find(" and ")?;
        (&rest[..pos], &rest[pos + 5..])
    } else {
        return None;
    };

    let (start_date, start_time) = parse_date_time(start_str.trim(), today)?;
    let (end_date, end_time) = parse_date_time(end_str.trim(), today)?;

    Some(ParsedDate {
        date: start_date,
        time: start_time,
        end_date: Some(end_date),
        end_time,
        is_date_range: true,
        duration_minutes: None,
    })
}

fn extract_duration(s: &str) -> (String, Option<i64>, Option<i64>) {
    let sl = s.to_lowercase();
    if let Some(pos) = sl.rfind(" for ") {
        let after = sl[pos + 5..].trim();
        let parts: Vec<&str> = after.splitn(2, ' ').collect();
        if let (Some(n_str), Some(unit)) = (parts.first(), parts.get(1)) {
            if let Ok(n) = n_str.parse::<i64>() {
                let unit = unit.trim();
                let base = s[..pos].to_string();
                match unit {
                    "minute" | "minutes" | "min" | "mins" | "m" => {
                        return (base, Some(n), None);
                    }
                    "hour" | "hours" | "hr" | "hrs" | "h" => {
                        return (base, Some(n * 60), None);
                    }
                    "day" | "days" | "d" => {
                        return (base, None, Some(n));
                    }
                    "week" | "weeks" | "wk" | "wks" | "w" => {
                        return (base, None, Some(n * 7));
                    }
                    _ => {}
                }
            }
        }
    }
    (s.to_string(), None, None)
}

fn parse_shorthand(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let s = s.trim();
    if s.len() < 3 {
        return None;
    }
    let first = s.chars().next()?;
    if first != '+' && first != '-' {
        return None;
    }
    let sign: i64 = if first == '+' { 1 } else { -1 };
    let rest = &s[1..];
    let last = rest.chars().last()?;
    if !last.is_alphabetic() {
        return None;
    }
    let n_str = &rest[..rest.len() - last.len_utf8()];
    let n: i64 = n_str.parse().ok()?;
    let amount = n * sign;
    match last.to_ascii_lowercase() {
        'd' => Some(today + Duration::days(amount)),
        'w' => Some(today + Duration::weeks(amount)),
        _ => None,
    }
}

fn parse_date_time(s: &str, today: NaiveDate) -> Option<(NaiveDate, Option<(u32, u32)>)> {
    let sl = s.to_lowercase();
    let sl = sl.trim();

    // Relative keywords: today, tomorrow, yesterday, tonight
    for (kw, offset) in &[
        ("tonight", 0i64),
        ("today", 0),
        ("tomorrow", 1),
        ("yesterday", -1),
    ] {
        if sl == *kw {
            return Some((today + Duration::days(*offset), None));
        }
        let with_space = format!("{} ", kw);
        if sl.starts_with(&with_space) {
            let date = today + Duration::days(*offset);
            let rest = strip_at_prefix(sl[kw.len()..].trim());
            return Some((date, parse_time(rest)));
        }
    }

    // "next <weekday>" or "this <weekday>"
    for prefix in &["next ", "this "] {
        if sl.starts_with(prefix) {
            let rest = s[prefix.len()..].trim();
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if let Some(wd) = parse_weekday(&parts[0].to_lowercase()) {
                let force_next = *prefix == "next ";
                let date = next_weekday_from(today, wd, force_next);
                let time_str = parts
                    .get(1)
                    .map(|t| strip_at_prefix(t.trim()))
                    .unwrap_or("");
                return Some((
                    date,
                    if time_str.is_empty() {
                        None
                    } else {
                        parse_time(time_str)
                    },
                ));
            }
        }
    }

    // "<weekday>" or "<weekday> <time>"
    {
        let parts: Vec<&str> = s.trim().splitn(2, ' ').collect();
        if let Some(wd) = parse_weekday(&parts[0].to_lowercase()) {
            let date = next_weekday_from(today, wd, false);
            let time_str = parts
                .get(1)
                .map(|t| strip_at_prefix(t.trim()))
                .unwrap_or("");
            return Some((
                date,
                if time_str.is_empty() {
                    None
                } else {
                    parse_time(time_str)
                },
            ));
        }
    }

    // "in N days/weeks/months"
    if sl.starts_with("in ") {
        let rest = &sl[3..];
        let parts: Vec<&str> = rest.splitn(3, ' ').collect();
        if let (Some(n_str), Some(unit)) = (parts.first(), parts.get(1)) {
            if let Ok(n) = n_str.parse::<i64>() {
                let days = match unit.trim().trim_end_matches('s') {
                    "day" | "d" => n,
                    "week" | "wk" | "w" => n * 7,
                    "month" | "mo" => n * 30,
                    _ => return None,
                };
                let date = today + Duration::days(days);
                let time_str = parts
                    .get(2)
                    .map(|t| strip_at_prefix(t.trim()))
                    .unwrap_or("");
                return Some((
                    date,
                    if time_str.is_empty() {
                        None
                    } else {
                        parse_time(time_str)
                    },
                ));
            }
        }
    }

    // Just a time (today)
    if let Some(t) = parse_time(sl) {
        return Some((today, Some(t)));
    }

    // ISO date: YYYY-MM-DD
    if let Ok(d) = NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d") {
        return Some((d, None));
    }

    // M/D/YYYY or M/D/YY
    for fmt in &["%m/%d/%Y", "%m/%d/%y"] {
        if let Ok(d) = NaiveDate::parse_from_str(s.trim(), fmt) {
            return Some((d, None));
        }
    }

    // M/D (current or next year)
    if let Ok(d) = NaiveDate::parse_from_str(s.trim(), "%m/%d") {
        return Some((adjust_year(d, today), None));
    }

    // "Mar 15", "March 15", "Mar 15 2026", etc.
    if let Some(d) = parse_month_day(s.trim(), today) {
        return Some((d, None));
    }

    None
}

fn strip_at_prefix<'a>(s: &'a str) -> &'a str {
    if let Some(rest) = s.strip_prefix("at ") {
        rest.trim()
    } else {
        s
    }
}

fn parse_time(s: &str) -> Option<(u32, u32)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // HH:MM (24h)
    if s.len() == 5 && s.chars().nth(2) == Some(':') {
        if let (Ok(h), Ok(m)) = (s[..2].parse::<u32>(), s[3..].parse::<u32>()) {
            if h < 24 && m < 60 {
                return Some((h, m));
            }
        }
    }

    // H:MM or HH:MM (24h, no strict length check)
    if let Some(colon) = s.find(':') {
        let h_str = &s[..colon];
        let rest = &s[colon + 1..];
        let sl = rest.to_lowercase();
        let is_pm = sl.ends_with("pm");
        let is_am = sl.ends_with("am");
        let m_str = if is_pm || is_am {
            &rest[..rest.len() - 2]
        } else {
            rest
        };
        if let (Ok(h), Ok(m)) = (h_str.parse::<u32>(), m_str.parse::<u32>()) {
            if m < 60 {
                let h24 = if is_pm {
                    if h == 12 {
                        12
                    } else {
                        (h + 12) % 24
                    }
                } else if is_am {
                    if h == 12 {
                        0
                    } else {
                        h
                    }
                } else {
                    h
                };
                if h24 < 24 {
                    return Some((h24, m));
                }
            }
        }
    }

    // Npm or Npm (no colon)
    let sl = s.to_lowercase();
    let is_pm = sl.ends_with("pm");
    let is_am = sl.ends_with("am");
    if is_pm || is_am {
        let h_str = &sl[..sl.len() - 2].trim();
        if let Ok(h) = h_str.parse::<u32>() {
            let h24 = if is_pm {
                if h == 12 {
                    12
                } else {
                    (h + 12) % 24
                }
            } else {
                if h == 12 {
                    0
                } else {
                    h
                }
            };
            if h24 < 24 {
                return Some((h24, 0));
            }
        }
    }

    None
}

fn parse_weekday(s: &str) -> Option<Weekday> {
    match s {
        "monday" | "mon" => Some(Weekday::Mon),
        "tuesday" | "tue" | "tues" => Some(Weekday::Tue),
        "wednesday" | "wed" => Some(Weekday::Wed),
        "thursday" | "thu" | "thur" | "thurs" => Some(Weekday::Thu),
        "friday" | "fri" => Some(Weekday::Fri),
        "saturday" | "sat" => Some(Weekday::Sat),
        "sunday" | "sun" => Some(Weekday::Sun),
        _ => None,
    }
}

fn next_weekday_from(today: NaiveDate, target: Weekday, force_next: bool) -> NaiveDate {
    let today_n = today.weekday().num_days_from_monday() as i64;
    let target_n = target.num_days_from_monday() as i64;
    let mut days = target_n - today_n;
    if days < 0 || (days == 0 && force_next) {
        days += 7;
    }
    today + Duration::days(days)
}

fn adjust_year(d: NaiveDate, today: NaiveDate) -> NaiveDate {
    let year = today.year();
    let candidate = d.with_year(year).unwrap_or(d);
    if candidate < today {
        candidate.with_year(year + 1).unwrap_or(candidate)
    } else {
        candidate
    }
}

fn parse_month_day(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    for fmt in &[
        "%b %d %Y", "%B %d %Y", "%b %d", "%B %d", "%d %b %Y", "%d %B %Y", "%d %b", "%d %B",
    ] {
        if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
            return Some(if fmt.contains("%Y") {
                d
            } else {
                adjust_year(d, today)
            });
        }
    }
    None
}

pub fn format_parsed_preview(parsed: &ParsedDate) -> String {
    let date_str = crate::domain::time::format_day_header(parsed.date);

    if parsed.is_date_range {
        if let Some(end) = parsed.end_date {
            let end_str = crate::domain::time::format_day_header(end);
            let days = (end - parsed.date).num_days();
            return format!("{} – {} ({} days)", date_str, end_str, days);
        }
        return date_str;
    }

    let time_str = if let Some((h, m)) = parsed.time {
        let ampm = if h < 12 { "AM" } else { "PM" };
        let h12 = match h {
            0 => 12,
            h if h > 12 => h - 12,
            h => h,
        };
        if let Some((eh, em)) = parsed.end_time {
            let eampm = if eh < 12 { "AM" } else { "PM" };
            let eh12 = match eh {
                0 => 12,
                h if h > 12 => h - 12,
                h => h,
            };
            format!(
                " at {:02}:{:02} {} – {:02}:{:02} {}",
                h12, m, ampm, eh12, em, eampm
            )
        } else {
            format!(" at {:02}:{:02} {}", h12, m, ampm)
        }
    } else {
        " (all day)".to_string()
    };

    format!("{}{}", date_str, time_str)
}
