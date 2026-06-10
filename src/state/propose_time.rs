use crate::domain::event::CalEvent;
use crate::domain::natural_date::{
    current_duration, format_parsed_preview, interpret_parsed_date_time, parse_natural_date,
    shift_end_to_preserve_duration, InterpretedDateTime,
};

pub const PT_FIELD_WHEN: usize = 0;
pub const PT_FIELD_START_DATE: usize = 1;
pub const PT_FIELD_START_TIME: usize = 2;
pub const PT_FIELD_END_DATE: usize = 3;
pub const PT_FIELD_END_TIME: usize = 4;
pub const PT_FIELD_MESSAGE: usize = 5;
pub const PT_FIELD_COUNT: usize = 6;

#[derive(Debug, Clone)]
pub struct ProposeTimeState {
    pub event_id: String,
    pub event_title: String,
    pub organizer: String,
    pub is_all_day: bool,

    pub start_date: String,
    pub start_time: String,
    pub end_date: String,
    pub end_time: String,
    pub message: String,

    // Natural-language when field
    pub when_input: String,
    pub when_preview: Option<String>,

    pub active_field: usize,

    start_edit_duration: Option<chrono::Duration>,
}

impl ProposeTimeState {
    pub fn from_event(event: &CalEvent) -> Self {
        let is_all_day = event.is_all_day();

        let start_date = if let Some(ref dt) = event.start.date_time {
            chrono::DateTime::parse_from_rfc3339(dt)
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default()
        } else {
            event.start.date.clone().unwrap_or_default()
        };

        let start_time = if let Some(ref dt) = event.start.date_time {
            chrono::DateTime::parse_from_rfc3339(dt)
                .map(|d| d.format("%H:%M").to_string())
                .unwrap_or_else(|_| "09:00".to_string())
        } else {
            "09:00".to_string()
        };

        let end_date = if let Some(ref dt) = event.end.date_time {
            chrono::DateTime::parse_from_rfc3339(dt)
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|_| start_date.clone())
        } else {
            event.end.date.clone().unwrap_or_else(|| start_date.clone())
        };

        let end_time = if let Some(ref dt) = event.end.date_time {
            chrono::DateTime::parse_from_rfc3339(dt)
                .map(|d| d.format("%H:%M").to_string())
                .unwrap_or_else(|_| "10:00".to_string())
        } else {
            "10:00".to_string()
        };

        let organizer = event
            .organizer
            .as_ref()
            .map(|o| o.display_name.as_deref().unwrap_or(&o.email).to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        Self {
            event_id: event.id.clone(),
            event_title: event.display_title().to_string(),
            organizer,
            is_all_day,
            start_date,
            start_time,
            end_date,
            end_time,
            message: String::new(),
            when_input: String::new(),
            when_preview: None,
            active_field: PT_FIELD_WHEN,
            start_edit_duration: None,
        }
    }

    pub fn next_field(&mut self) {
        self.active_field = (self.active_field + 1) % PT_FIELD_COUNT;
        self.reset_start_edit_duration_if_needed();
    }

    pub fn prev_field(&mut self) {
        self.active_field = if self.active_field == 0 {
            PT_FIELD_COUNT - 1
        } else {
            self.active_field - 1
        };
        self.reset_start_edit_duration_if_needed();
    }

    pub fn active_string(&mut self) -> &mut String {
        match self.active_field {
            PT_FIELD_WHEN => &mut self.when_input,
            PT_FIELD_START_DATE => &mut self.start_date,
            PT_FIELD_START_TIME => &mut self.start_time,
            PT_FIELD_END_DATE => &mut self.end_date,
            PT_FIELD_END_TIME => &mut self.end_time,
            PT_FIELD_MESSAGE => &mut self.message,
            _ => &mut self.message,
        }
    }

    pub fn push_char(&mut self, c: char) {
        let field = self.active_field;
        let duration = self.duration_before_start_edit();
        let s = self.active_string();
        s.push(c);
        if field == PT_FIELD_WHEN {
            self.refresh_when_preview();
        } else {
            self.preserve_end_after_start_edit(duration);
        }
    }

    pub fn pop_char(&mut self) {
        let field = self.active_field;
        let duration = self.duration_before_start_edit();
        let s = self.active_string();
        s.pop();
        if field == PT_FIELD_WHEN {
            self.refresh_when_preview();
        } else {
            self.preserve_end_after_start_edit(duration);
        }
    }

    fn refresh_when_preview(&mut self) {
        if self.when_input.is_empty() {
            self.when_preview = None;
        } else {
            self.when_preview =
                parse_natural_date(&self.when_input.clone()).map(|p| format_parsed_preview(&p));
        }
    }

    pub fn apply_when_input(&mut self) {
        if let Some(preview) = self.when_interpretation() {
            self.is_all_day = preview.is_all_day;
            self.start_date = preview.start_date;
            self.start_time = preview.start_time;
            self.end_date = preview.end_date;
            self.end_time = preview.end_time;
            self.when_input.clear();
            self.when_preview = None;
            self.active_field = PT_FIELD_MESSAGE;
        }
    }

    pub fn when_interpretation(&self) -> Option<InterpretedDateTime> {
        let parsed = parse_natural_date(&self.when_input)?;
        Some(interpret_parsed_date_time(
            &parsed,
            &self.start_date,
            &self.start_time,
            &self.end_date,
            &self.end_time,
        ))
    }

    fn duration_before_start_edit(&mut self) -> Option<chrono::Duration> {
        if self.is_all_day
            || (self.active_field != PT_FIELD_START_DATE
                && self.active_field != PT_FIELD_START_TIME)
        {
            self.start_edit_duration = None;
            return None;
        }
        if self.start_edit_duration.is_none() {
            self.start_edit_duration = current_duration(
                &self.start_date,
                &self.start_time,
                &self.end_date,
                &self.end_time,
            );
        }
        self.start_edit_duration
    }

    fn preserve_end_after_start_edit(&mut self, duration: Option<chrono::Duration>) {
        let Some(duration) = duration else {
            return;
        };
        if let Some((end_date, end_time)) =
            shift_end_to_preserve_duration(&self.start_date, &self.start_time, duration)
        {
            self.end_date = end_date;
            self.end_time = end_time;
        }
    }

    fn reset_start_edit_duration_if_needed(&mut self) {
        if self.active_field != PT_FIELD_START_DATE && self.active_field != PT_FIELD_START_TIME {
            self.start_edit_duration = None;
        }
    }
}
