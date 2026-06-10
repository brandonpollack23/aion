use std::collections::HashMap;

use crate::domain::event::CalEvent;

#[derive(Debug, Clone)]
pub struct Contact {
    pub email: String,
    pub display_name: Option<String>,
}

/// Load contacts from ~/.config/aion/contacts.json.
/// File format: {"email@example.com": "Display Name", ...}
pub fn load_contacts_from_file() -> Vec<Contact> {
    let path = match dirs::config_dir() {
        Some(p) => p.join("aion").join("contacts.json"),
        None => return Vec::new(),
    };

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let map: HashMap<String, String> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };

    map.into_iter()
        .map(|(email, name)| Contact {
            email,
            display_name: if name.is_empty() { None } else { Some(name) },
        })
        .collect()
}

/// Extract unique contacts from event attendees.
pub fn extract_contacts_from_events(events: &HashMap<String, CalEvent>) -> Vec<Contact> {
    let mut seen: HashMap<String, Option<String>> = HashMap::new();

    for event in events.values() {
        if let Some(attendees) = &event.attendees {
            for attendee in attendees {
                // Skip self-attendee entries
                if attendee.is_self == Some(true) {
                    continue;
                }
                seen.entry(attendee.email.clone())
                    .or_insert_with(|| attendee.display_name.clone());
            }
        }
    }

    seen.into_iter()
        .map(|(email, display_name)| Contact {
            email,
            display_name,
        })
        .collect()
}

/// Merge contacts from file and events, deduplicated by email (file names take precedence).
pub fn merged_contacts(events: &HashMap<String, CalEvent>) -> Vec<Contact> {
    let file_contacts = load_contacts_from_file();
    let event_contacts = extract_contacts_from_events(events);

    let mut by_email: HashMap<String, Contact> = HashMap::new();

    // Event contacts first (lower priority)
    for c in event_contacts {
        by_email.insert(c.email.clone(), c);
    }

    // File contacts override
    for c in file_contacts {
        by_email.insert(c.email.clone(), c);
    }

    let mut result: Vec<Contact> = by_email.into_values().collect();
    result.sort_by(|a, b| {
        let a_name = a.display_name.as_deref().unwrap_or(&a.email);
        let b_name = b.display_name.as_deref().unwrap_or(&b.email);
        a_name.cmp(b_name)
    });
    result
}
