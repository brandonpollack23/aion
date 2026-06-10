use crate::api::contacts::Contact;
use crate::api::gcal::BusyPeriod;
use crate::domain::free_slots::TimeSlot;

pub const DURATION_OPTIONS: &[i64] = &[15, 30, 45, 60, 90, 120];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeetWithStep {
    People,
    Slots,
}

#[derive(Debug, Clone)]
pub struct MeetWithState {
    pub step: MeetWithStep,

    // People step
    pub all_contacts: Vec<Contact>,
    pub selected_people: Vec<Contact>,
    pub search_query: String,
    pub list_cursor: usize, // index in filtered contact list

    // Slots step
    pub duration_minutes: i64,
    pub slots: Vec<TimeSlot>,
    pub busy_periods: Vec<BusyPeriod>,
    pub slots_loading: bool,
    pub slots_cursor: usize,
}

impl MeetWithState {
    pub fn new(contacts: Vec<Contact>) -> Self {
        Self {
            step: MeetWithStep::People,
            all_contacts: contacts,
            selected_people: Vec::new(),
            search_query: String::new(),
            list_cursor: 0,
            duration_minutes: 30,
            slots: Vec::new(),
            busy_periods: Vec::new(),
            slots_loading: false,
            slots_cursor: 0,
        }
    }

    pub fn filtered_contacts(&self) -> Vec<&Contact> {
        if self.search_query.is_empty() {
            self.all_contacts.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.all_contacts
                .iter()
                .filter(|c| {
                    c.email.to_lowercase().contains(&q)
                        || c.display_name
                            .as_deref()
                            .map_or(false, |n| n.to_lowercase().contains(&q))
                })
                .collect()
        }
    }

    pub fn is_selected(&self, email: &str) -> bool {
        self.selected_people.iter().any(|p| p.email == email)
    }

    pub fn toggle_contact(&mut self, email: &str) {
        if let Some(pos) = self.selected_people.iter().position(|p| p.email == email) {
            self.selected_people.remove(pos);
        } else {
            // Find in all_contacts for display_name
            if let Some(contact) = self.all_contacts.iter().find(|c| c.email == email).cloned() {
                self.selected_people.push(contact);
            } else {
                self.selected_people.push(Contact {
                    email: email.to_string(),
                    display_name: None,
                });
            }
        }
    }

    pub fn add_email(&mut self, email: &str) {
        if email.contains('@') && !self.is_selected(email) {
            self.selected_people.push(Contact {
                email: email.to_string(),
                display_name: None,
            });
        }
    }

    pub fn duration_display(&self) -> String {
        let m = self.duration_minutes;
        if m < 60 {
            format!("{}m", m)
        } else {
            let h = m / 60;
            let rem = m % 60;
            if rem == 0 {
                format!("{}h", h)
            } else {
                format!("{}h {}m", h, rem)
            }
        }
    }

    pub fn cycle_duration(&mut self, forward: bool) {
        let idx = DURATION_OPTIONS
            .iter()
            .position(|&d| d == self.duration_minutes)
            .unwrap_or(1);
        if forward {
            self.duration_minutes = DURATION_OPTIONS[(idx + 1).min(DURATION_OPTIONS.len() - 1)];
        } else {
            self.duration_minutes = DURATION_OPTIONS[idx.saturating_sub(1)];
        }
    }

    pub fn selected_emails(&self) -> Vec<String> {
        self.selected_people
            .iter()
            .map(|p| p.email.clone())
            .collect()
    }

    pub fn selected_names_display(&self) -> String {
        self.selected_people
            .iter()
            .map(|p| {
                p.display_name
                    .as_deref()
                    .unwrap_or_else(|| p.email.split('@').next().unwrap_or(&p.email))
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}
