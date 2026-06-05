use chrono::NaiveDate;
use std::collections::HashMap;

use crate::domain::event::CalEvent;
use crate::domain::time::{
    event_falls_on_day, get_duration_minutes, get_event_end, get_event_start,
    get_hour_bucket, get_minutes_from_midnight,
};

pub const SLOTS_PER_HOUR: i64 = 4;
pub const MINUTES_PER_SLOT: i64 = 15;
pub const TOTAL_SLOTS: i64 = 24 * SLOTS_PER_HOUR; // 96

#[derive(Debug, Clone)]
pub struct TimedEventLayout {
    pub event: CalEvent,
    pub start_minutes: i64, // 0-1440
    pub end_minutes: i64,   // 0-1440
    pub hour_bucket_start: i64,
    pub hour_bucket_end: i64,
    pub duration_minutes: i64,
    pub offset_in_hour: i64,
    pub column: usize,
    pub total_columns: usize,
}

#[derive(Debug, Clone)]
pub struct DayLayout {
    pub day: NaiveDate,
    pub timezone: String,
    pub all_day_events: Vec<CalEvent>,
    pub timed_events: Vec<TimedEventLayout>,
    pub hour_buckets: HashMap<i64, Vec<usize>>, // hour → indices into timed_events
}

impl DayLayout {
    pub fn get_events_for_slot(&self, slot_index: i64) -> Vec<&TimedEventLayout> {
        let slot_start = slot_index * MINUTES_PER_SLOT;
        let slot_end = slot_start + MINUTES_PER_SLOT;
        self.timed_events
            .iter()
            .filter(|l| l.start_minutes < slot_end && l.end_minutes > slot_start)
            .collect()
    }

    pub fn chronological_events(&self) -> Vec<&CalEvent> {
        let mut events: Vec<&CalEvent> = self.all_day_events.iter().collect();
        events.extend(self.timed_events.iter().map(|l| &l.event));
        events
    }

    pub fn find_nearest_event(&self, minute_of_day: i64) -> Option<&CalEvent> {
        if self.timed_events.is_empty() {
            return self.all_day_events.first();
        }
        let nearest = self.timed_events.iter().min_by_key(|l| {
            (l.start_minutes - minute_of_day).unsigned_abs()
        })?;
        Some(&nearest.event)
    }

    pub fn get_event_layout(&self, event_id: &str) -> Option<&TimedEventLayout> {
        self.timed_events.iter().find(|l| l.event.id == event_id)
    }

    pub fn max_columns_in_slot(&self, slot_index: i64) -> usize {
        self.get_events_for_slot(slot_index)
            .iter()
            .map(|l| l.total_columns)
            .max()
            .unwrap_or(1)
    }
}

pub fn layout_day(events: &[CalEvent], day: NaiveDate, tz: &str) -> DayLayout {
    let day_events: Vec<&CalEvent> = events
        .iter()
        .filter(|e| {
            !matches!(e.status, crate::domain::event::EventStatus::Cancelled)
                && event_falls_on_day(e, day, tz)
        })
        .collect();

    let mut all_day_events = Vec::new();
    let mut timed_raw: Vec<(&CalEvent, chrono::DateTime<chrono_tz::Tz>, chrono::DateTime<chrono_tz::Tz>)> = Vec::new();

    for event in day_events {
        if event.is_all_day() {
            all_day_events.push(event.clone());
        } else {
            let start = get_event_start(event, tz);
            let end = get_event_end(event, tz);
            if let (Some(s), Some(e)) = (start, end) {
                timed_raw.push((event, s, e));
            }
        }
    }

    // Sort by start time, then longer events first
    timed_raw.sort_by(|(_, a_start, a_end), (_, b_start, b_end)| {
        let start_cmp = a_start.cmp(b_start);
        if start_cmp != std::cmp::Ordering::Equal {
            return start_cmp;
        }
        // Longer events first
        let a_dur = (*a_end - *a_start).num_minutes();
        let b_dur = (*b_end - *b_start).num_minutes();
        b_dur.cmp(&a_dur)
    });

    use chrono::{Duration, TimeZone};
    let tz_parsed: chrono_tz::Tz = tz.parse().unwrap_or(chrono_tz::UTC);
    let day_start = tz_parsed
        .from_local_datetime(&day.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .unwrap_or_else(|| {
            tz_parsed.from_utc_datetime(&day.and_hms_opt(0, 0, 0).unwrap())
        });
    let day_end = day_start + Duration::days(1);
    let day_end_minutes = 24 * 60;

    let mut timed_layouts: Vec<TimedEventLayout> = Vec::new();

    for (event, start, end) in &timed_raw {
        let event_day_start = if *start < day_start { day_start } else { *start };
        let event_day_end = if *end > day_end { day_end } else { *end };

        let start_minutes = get_minutes_from_midnight(&event_day_start).max(0);
        let raw_end_minutes = get_minutes_from_midnight(&event_day_end);
        let end_minutes = raw_end_minutes.min(day_end_minutes);
        let hour_bucket_start = get_hour_bucket(&event_day_start);
        let hour_bucket_end = if end_minutes == 0 { 23 } else { ((end_minutes - 1) / 60).min(23) };
        let offset_in_hour = start_minutes % 60;

        timed_layouts.push(TimedEventLayout {
            event: (*event).clone(),
            start_minutes,
            end_minutes: if end_minutes == 0 { day_end_minutes } else { end_minutes },
            hour_bucket_start,
            hour_bucket_end: if raw_end_minutes == 0 { 23 } else { hour_bucket_end },
            duration_minutes: get_duration_minutes(&event_day_start, &event_day_end),
            offset_in_hour,
            column: 0,
            total_columns: 1,
        });
    }

    assign_columns(&mut timed_layouts);

    // Build hour buckets: hour → list of indices
    let mut hour_buckets: HashMap<i64, Vec<usize>> = (0..24).map(|h| (h, vec![])).collect();
    for (idx, layout) in timed_layouts.iter().enumerate() {
        if let Some(bucket) = hour_buckets.get_mut(&layout.hour_bucket_start) {
            bucket.push(idx);
        }
    }

    DayLayout {
        day,
        timezone: tz.to_string(),
        all_day_events,
        timed_events: timed_layouts,
        hour_buckets,
    }
}

fn events_overlap(a: &TimedEventLayout, b: &TimedEventLayout) -> bool {
    a.start_minutes < b.end_minutes && b.start_minutes < a.end_minutes
}

fn assign_columns(layouts: &mut Vec<TimedEventLayout>) {
    if layouts.is_empty() {
        return;
    }

    let n = layouts.len();
    let mut assigned = vec![false; n];
    let mut clusters: Vec<Vec<usize>> = Vec::new();

    for i in 0..n {
        if assigned[i] {
            continue;
        }

        let mut cluster = Vec::new();
        let mut queue = vec![i];

        while let Some(current) = queue.first().cloned() {
            queue.remove(0);
            if assigned[current] {
                continue;
            }
            assigned[current] = true;
            cluster.push(current);

            for j in 0..n {
                if !assigned[j] && events_overlap(&layouts[current], &layouts[j]) {
                    queue.push(j);
                }
            }
        }

        if !cluster.is_empty() {
            clusters.push(cluster);
        }
    }

    for cluster in clusters {
        // Sort cluster indices by start time, then duration descending
        let mut sorted = cluster.clone();
        sorted.sort_by(|&a, &b| {
            let start_cmp = layouts[a].start_minutes.cmp(&layouts[b].start_minutes);
            if start_cmp != std::cmp::Ordering::Equal {
                return start_cmp;
            }
            layouts[b].duration_minutes.cmp(&layouts[a].duration_minutes)
        });

        let mut column_ends: Vec<i64> = Vec::new();

        for &idx in &sorted {
            let start = layouts[idx].start_minutes;
            let mut col = 0;
            while col < column_ends.len() && column_ends[col] > start {
                col += 1;
            }
            layouts[idx].column = col;
            if col < column_ends.len() {
                column_ends[col] = layouts[idx].end_minutes;
            } else {
                column_ends.push(layouts[idx].end_minutes);
            }
        }

        let total_cols = column_ends.len();
        for &idx in &sorted {
            layouts[idx].total_columns = total_cols;
        }
    }
}

pub fn minutes_to_slot(minutes: i64) -> i64 {
    minutes / MINUTES_PER_SLOT
}
