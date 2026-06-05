use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Weekday};

use crate::api::gcal::BusyPeriod;

#[derive(Debug, Clone)]
pub struct TimeSlot {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub duration_minutes: i64,
}

pub struct FindSlotsOptions {
    pub min_duration_minutes: i64,
    pub working_hours_start: u32,
    pub working_hours_end: u32,
    pub include_weekends: bool,
    pub days_ahead: i64,
}

impl Default for FindSlotsOptions {
    fn default() -> Self {
        Self {
            min_duration_minutes: 30,
            working_hours_start: 9,
            working_hours_end: 18,
            include_weekends: false,
            days_ahead: 7,
        }
    }
}

fn parse_busy(periods: &[BusyPeriod]) -> Vec<(DateTime<Local>, DateTime<Local>)> {
    let mut parsed: Vec<(DateTime<Local>, DateTime<Local>)> = periods
        .iter()
        .filter_map(|p| {
            let start = chrono::DateTime::parse_from_rfc3339(&p.start)
                .ok()
                .map(|dt| dt.with_timezone(&Local))?;
            let end = chrono::DateTime::parse_from_rfc3339(&p.end)
                .ok()
                .map(|dt| dt.with_timezone(&Local))?;
            Some((start, end))
        })
        .collect();
    parsed.sort_by_key(|(s, _)| *s);
    parsed
}

fn merge_busy(
    mut periods: Vec<(DateTime<Local>, DateTime<Local>)>,
) -> Vec<(DateTime<Local>, DateTime<Local>)> {
    if periods.is_empty() {
        return periods;
    }
    periods.sort_by_key(|(s, _)| *s);
    let mut merged: Vec<(DateTime<Local>, DateTime<Local>)> = vec![periods[0]];
    for (start, end) in periods.into_iter().skip(1) {
        let last = merged.last_mut().unwrap();
        if start <= last.1 {
            if end > last.1 {
                last.1 = end;
            }
        } else {
            merged.push((start, end));
        }
    }
    merged
}

fn naive_date_from_offset(base: NaiveDate, offset_days: i64) -> NaiveDate {
    base + Duration::days(offset_days)
}

pub fn find_meeting_slots(busy_periods: &[BusyPeriod], opts: &FindSlotsOptions) -> Vec<TimeSlot> {
    let now = Local::now();
    let range_end = now + Duration::days(opts.days_ahead);

    let busy = merge_busy(parse_busy(busy_periods));

    let mut slots: Vec<TimeSlot> = Vec::new();
    let start_date = now.date_naive();

    for day_offset in 0..=opts.days_ahead {
        let current_day = naive_date_from_offset(start_date, day_offset);

        let weekday = current_day.weekday();
        if !opts.include_weekends && (weekday == Weekday::Sat || weekday == Weekday::Sun) {
            continue;
        }

        let day_start_naive = current_day
            .and_time(NaiveTime::from_hms_opt(opts.working_hours_start, 0, 0).unwrap());
        let day_end_naive = current_day
            .and_time(NaiveTime::from_hms_opt(opts.working_hours_end, 0, 0).unwrap());

        let day_start = match Local.from_local_datetime(&day_start_naive).earliest() {
            Some(dt) => dt,
            None => continue,
        };
        let day_end = match Local.from_local_datetime(&day_end_naive).earliest() {
            Some(dt) => dt,
            None => continue,
        };

        if day_end > range_end {
            break;
        }

        let effective_start = if day_start < now { now } else { day_start };

        if effective_start >= day_end {
            continue;
        }

        let relevant_busy: Vec<_> = busy
            .iter()
            .filter(|(s, e)| *s < day_end && *e > effective_start)
            .collect();

        let free_intervals = subtract_busy(effective_start, day_end, &relevant_busy);

        for (fs, fe) in free_intervals {
            let total_mins = (fe - fs).num_minutes();
            if total_mins < opts.min_duration_minutes {
                continue;
            }
            let mut slot_start = fs;
            while slot_start + Duration::minutes(opts.min_duration_minutes) <= fe {
                let slot_end = slot_start + Duration::minutes(opts.min_duration_minutes);
                slots.push(TimeSlot {
                    start: slot_start,
                    end: slot_end,
                    duration_minutes: opts.min_duration_minutes,
                });
                slot_start = slot_end;
            }
        }
    }

    slots
}

fn subtract_busy(
    range_start: DateTime<Local>,
    range_end: DateTime<Local>,
    busy: &[&(DateTime<Local>, DateTime<Local>)],
) -> Vec<(DateTime<Local>, DateTime<Local>)> {
    let mut free: Vec<(DateTime<Local>, DateTime<Local>)> = Vec::new();
    let mut current = range_start;

    for (bs, be) in busy {
        if *bs > current {
            let gap_end = (*bs).min(range_end);
            if gap_end > current {
                free.push((current, gap_end));
            }
        }
        if *be > current {
            current = *be;
        }
        if current >= range_end {
            break;
        }
    }

    if current < range_end {
        free.push((current, range_end));
    }

    free
}

pub fn combine_busy_periods(
    by_person: &std::collections::HashMap<String, Vec<BusyPeriod>>,
) -> Vec<BusyPeriod> {
    let mut all: Vec<(DateTime<Local>, DateTime<Local>)> = Vec::new();
    for periods in by_person.values() {
        all.extend(parse_busy(periods));
    }
    let merged = merge_busy(all);
    merged
        .into_iter()
        .map(|(s, e)| BusyPeriod {
            start: s.to_rfc3339(),
            end: e.to_rfc3339(),
        })
        .collect()
}
