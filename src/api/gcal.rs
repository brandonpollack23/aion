use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::auth::tokens::get_valid_access_token;
use crate::domain::event::{
    Attendee, CalEvent, EventStatus, EventType, Organizer, Reminders, ResponseStatus, TimeObject,
};

const GCAL_BASE: &str = "https://www.googleapis.com/calendar/v3";

// ── Calendar list ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarListEntry {
    pub id: String,
    pub summary: String,
    #[serde(rename = "backgroundColor")]
    pub background_color: Option<String>,
    #[serde(rename = "foregroundColor")]
    pub foreground_color: Option<String>,
    pub primary: Option<bool>,
    #[serde(rename = "accessRole")]
    pub access_role: String,
    // Injected by us after fetching
    #[serde(skip)]
    pub account_email: String,
}

#[derive(Deserialize)]
struct CalendarListResponse {
    items: Option<Vec<CalendarListEntry>>,
}

// ── Raw Google event types ─────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GRawTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
    #[serde(rename = "timeZone")]
    time_zone: Option<String>,
}

#[derive(Deserialize)]
struct GRawAttendee {
    email: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "responseStatus")]
    response_status: Option<String>,
    organizer: Option<bool>,
    #[serde(rename = "self")]
    is_self: Option<bool>,
}

#[derive(Deserialize)]
struct GRawOrganizer {
    email: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "self")]
    is_self: Option<bool>,
}

#[derive(Deserialize)]
struct GRawReminder {
    method: String,
    minutes: i64,
}

#[derive(Deserialize)]
struct GRawReminders {
    #[serde(rename = "useDefault")]
    use_default: Option<bool>,
    overrides: Option<Vec<GRawReminder>>,
}

#[derive(Deserialize)]
struct GRawEvent {
    id: String,
    status: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    location: Option<String>,
    #[serde(rename = "htmlLink")]
    html_link: Option<String>,
    start: Option<GRawTime>,
    end: Option<GRawTime>,
    attendees: Option<Vec<GRawAttendee>>,
    organizer: Option<GRawOrganizer>,
    recurrence: Option<Vec<String>>,
    #[serde(rename = "recurringEventId")]
    recurring_event_id: Option<String>,
    #[serde(rename = "hangoutLink")]
    hangout_link: Option<String>,
    reminders: Option<GRawReminders>,
    #[serde(rename = "eventType")]
    event_type: Option<String>,
}

#[derive(Deserialize)]
struct EventsListResponse {
    items: Option<Vec<GRawEvent>>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
    #[serde(rename = "nextSyncToken")]
    next_sync_token: Option<String>,
}

// ── Incremental sync result ────────────────────────────────────────────────────

pub struct IncrementalSyncResult {
    pub changed: Vec<CalEvent>,
    pub deleted: Vec<String>, // composite IDs
    pub next_sync_token: Option<String>,
    pub full_sync_required: bool,
}

// ── ID helpers ─────────────────────────────────────────────────────────────────

pub fn make_composite_id(account_email: &str, calendar_id: &str, google_id: &str) -> String {
    format!("{}:{}:{}", account_email, calendar_id, google_id)
}

pub fn extract_google_id(composite_id: &str) -> String {
    // "account@gmail.com:calendarId:googleId" → "googleId"
    let parts: Vec<&str> = composite_id.splitn(3, ':').collect();
    if parts.len() == 3 {
        parts[2].to_string()
    } else {
        composite_id.to_string()
    }
}

// ── Event conversion ──────────────────────────────────────────────────────────

fn parse_response_status(s: &str) -> Option<ResponseStatus> {
    match s {
        "accepted" => Some(ResponseStatus::Accepted),
        "declined" => Some(ResponseStatus::Declined),
        "tentative" => Some(ResponseStatus::Tentative),
        "needsAction" => Some(ResponseStatus::NeedsAction),
        _ => None,
    }
}

fn raw_to_cal_event(raw: &GRawEvent, account_email: &str, calendar_id: &str) -> CalEvent {
    let google_id = &raw.id;
    let composite_id = make_composite_id(account_email, calendar_id, google_id);

    let status = match raw.status.as_deref() {
        Some("tentative") => EventStatus::Tentative,
        Some("cancelled") => EventStatus::Cancelled,
        _ => EventStatus::Confirmed,
    };

    let event_type = raw.event_type.as_deref().map(|s| match s {
        "outOfOffice" => EventType::OutOfOffice,
        "focusTime" => EventType::FocusTime,
        "birthday" => EventType::Birthday,
        "workingLocation" => EventType::WorkingLocation,
        _ => EventType::Default,
    });

    let start = raw.start.as_ref().map(|t| TimeObject {
        date: t.date.clone(),
        date_time: t.date_time.clone(),
        time_zone: t.time_zone.clone(),
    }).unwrap_or(TimeObject { date: None, date_time: None, time_zone: None });

    let end = raw.end.as_ref().map(|t| TimeObject {
        date: t.date.clone(),
        date_time: t.date_time.clone(),
        time_zone: t.time_zone.clone(),
    }).unwrap_or(TimeObject { date: None, date_time: None, time_zone: None });

    let attendees = raw.attendees.as_ref().map(|list| {
        list.iter().map(|a| Attendee {
            email: a.email.clone(),
            display_name: a.display_name.clone(),
            response_status: a.response_status.as_deref().and_then(parse_response_status),
            organizer: a.organizer,
            is_self: a.is_self,
        }).collect()
    });

    let organizer = raw.organizer.as_ref().map(|o| Organizer {
        email: o.email.clone(),
        display_name: o.display_name.clone(),
        is_self: o.is_self,
    });

    let reminders = raw.reminders.as_ref().map(|r| {
        use crate::domain::event::{Reminder, ReminderMethod};
        Reminders {
            use_default: r.use_default,
            overrides: r.overrides.as_ref().map(|list| {
                list.iter().map(|rm| Reminder {
                    method: match rm.method.as_str() {
                        "email" => ReminderMethod::Email,
                        _ => ReminderMethod::Popup,
                    },
                    minutes: rm.minutes,
                }).collect()
            }),
        }
    });

    CalEvent {
        id: composite_id,
        summary: raw.summary.clone().unwrap_or_default(),
        description: raw.description.clone(),
        location: raw.location.clone(),
        html_link: raw.html_link.clone(),
        status,
        event_type,
        start,
        end,
        attendees,
        organizer,
        recurrence: raw.recurrence.clone(),
        recurring_event_id: raw.recurring_event_id.clone(),
        original_start_time: None,
        hangout_link: raw.hangout_link.clone(),
        reminders,
        account_email: Some(account_email.to_string()),
        calendar_id: Some(calendar_id.to_string()),
    }
}

// ── GcalClient ────────────────────────────────────────────────────────────────

pub struct GcalClient {
    http: reqwest::Client,
    client_id: String,
    client_secret: String,
}

impl GcalClient {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            client_id,
            client_secret,
        }
    }

    async fn access_token(&self, account_email: &str) -> Result<String> {
        get_valid_access_token(account_email, &self.client_id, &self.client_secret)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No access token for {}", account_email))
    }

    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        account_email: &str,
        path: &str,
    ) -> Result<T> {
        let token = self.access_token(account_email).await?;
        let url = format!("{}{}", GCAL_BASE, path);
        let res = self.http.get(&url).bearer_auth(&token).send().await?;
        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("GCal API {} {}: {}", status.as_u16(), path, text);
        }
        Ok(res.json::<T>().await?)
    }

    async fn post_json<T: for<'de> Deserialize<'de>>(
        &self,
        account_email: &str,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T> {
        let token = self.access_token(account_email).await?;
        let url = format!("{}{}", GCAL_BASE, path);
        let res = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .json(body)
            .send()
            .await?;
        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("GCal API {} {}: {}", status.as_u16(), path, text);
        }
        Ok(res.json::<T>().await?)
    }

    async fn patch_json<T: for<'de> Deserialize<'de>>(
        &self,
        account_email: &str,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T> {
        let token = self.access_token(account_email).await?;
        let url = format!("{}{}", GCAL_BASE, path);
        let res = self
            .http
            .patch(&url)
            .bearer_auth(&token)
            .json(body)
            .send()
            .await?;
        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("GCal API {} {}: {}", status.as_u16(), path, text);
        }
        Ok(res.json::<T>().await?)
    }

    async fn delete(&self, account_email: &str, path: &str) -> Result<()> {
        let token = self.access_token(account_email).await?;
        let url = format!("{}{}", GCAL_BASE, path);
        let res = self
            .http
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await?;
        if !res.status().is_success() && res.status().as_u16() != 204 {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("GCal API {} {}: {}", status.as_u16(), path, text);
        }
        Ok(())
    }

    // ── Public API ─────────────────────────────────────────────────────────────

    pub async fn get_calendar_list(&self, account_email: &str) -> Result<Vec<CalendarListEntry>> {
        let resp: CalendarListResponse = self
            .get(account_email, "/users/me/calendarList")
            .await?;
        let mut items = resp.items.unwrap_or_default();
        for item in &mut items {
            item.account_email = account_email.to_string();
        }
        Ok(items)
    }

    /// Full sync: fetch all events in [time_min, time_max]. Returns (events, sync_token).
    pub async fn fetch_events(
        &self,
        account_email: &str,
        calendar_id: &str,
        time_min: &str,
        time_max: &str,
    ) -> Result<(Vec<CalEvent>, Option<String>)> {
        let cal_enc = urlencoding::encode(calendar_id);
        let mut all_events: Vec<CalEvent> = Vec::new();
        let mut page_token: Option<String> = None;

        let sync_token = loop {
            let mut path = format!(
                "/calendars/{}/events?timeMin={}&timeMax={}&singleEvents=true&maxResults=250&orderBy=startTime",
                cal_enc, urlencoding::encode(time_min), urlencoding::encode(time_max)
            );
            if let Some(ref pt) = page_token {
                path.push_str(&format!("&pageToken={}", pt));
            }

            let resp: EventsListResponse = self.get(account_email, &path).await?;

            for raw in resp.items.unwrap_or_default() {
                if raw.status.as_deref() == Some("cancelled") {
                    continue;
                }
                all_events.push(raw_to_cal_event(&raw, account_email, calendar_id));
            }

            page_token = resp.next_page_token;
            if page_token.is_none() {
                break resp.next_sync_token;
            }
        };

        Ok((all_events, sync_token))
    }

    /// Incremental sync using an existing sync token.
    pub async fn incremental_sync(
        &self,
        account_email: &str,
        calendar_id: &str,
        sync_token: &str,
    ) -> Result<IncrementalSyncResult> {
        let cal_enc = urlencoding::encode(calendar_id);
        let mut changed: Vec<CalEvent> = Vec::new();
        let mut deleted: Vec<String> = Vec::new();
        let mut page_token: Option<String> = None;

        let next_sync_token = loop {
            let mut path = format!(
                "/calendars/{}/events?syncToken={}&maxResults=250",
                cal_enc,
                urlencoding::encode(sync_token)
            );
            if let Some(ref pt) = page_token {
                path.push_str(&format!("&pageToken={}", pt));
            }

            let token = self.access_token(account_email).await?;
            let url = format!("{}{}", GCAL_BASE, path);
            let res = self.http.get(&url).bearer_auth(&token).send().await?;

            // 410 Gone = sync token invalid, need full sync
            if res.status().as_u16() == 410 {
                return Ok(IncrementalSyncResult {
                    changed: vec![],
                    deleted: vec![],
                    next_sync_token: None,
                    full_sync_required: true,
                });
            }

            if !res.status().is_success() {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                anyhow::bail!("Incremental sync {} {}: {}", status.as_u16(), calendar_id, text);
            }

            let resp: EventsListResponse = res.json().await?;

            for raw in resp.items.unwrap_or_default() {
                let composite_id =
                    make_composite_id(account_email, calendar_id, &raw.id);
                if raw.status.as_deref() == Some("cancelled") {
                    deleted.push(composite_id);
                } else {
                    changed.push(raw_to_cal_event(&raw, account_email, calendar_id));
                }
            }

            page_token = resp.next_page_token;
            if page_token.is_none() {
                break resp.next_sync_token;
            }
        };

        Ok(IncrementalSyncResult {
            changed,
            deleted,
            next_sync_token,
            full_sync_required: false,
        })
    }

    pub async fn create_event(
        &self,
        account_email: &str,
        calendar_id: &str,
        event: &CalEvent,
    ) -> Result<CalEvent> {
        self.create_event_inner(account_email, calendar_id, event, false).await
    }

    pub async fn create_event_with_meet(
        &self,
        account_email: &str,
        calendar_id: &str,
        event: &CalEvent,
    ) -> Result<CalEvent> {
        self.create_event_inner(account_email, calendar_id, event, true).await
    }

    async fn create_event_inner(
        &self,
        account_email: &str,
        calendar_id: &str,
        event: &CalEvent,
        add_meet: bool,
    ) -> Result<CalEvent> {
        let cal_enc = urlencoding::encode(calendar_id);
        let mut path = format!("/calendars/{}/events", cal_enc);
        if add_meet {
            path.push_str("?conferenceDataVersion=1");
        }
        let body = event_to_json_with_opts(event, add_meet);
        let raw: GRawEvent = self.post_json(account_email, &path, &body).await?;
        Ok(raw_to_cal_event(&raw, account_email, calendar_id))
    }

    /// Update the self-attendee's RSVP status for an event.
    pub async fn update_attendance(
        &self,
        account_email: &str,
        calendar_id: &str,
        composite_id: &str,
        response_status: &str,
        current_attendees: &[crate::domain::event::Attendee],
    ) -> Result<()> {
        use serde_json::json;

        let google_id = extract_google_id(composite_id);
        let cal_enc = urlencoding::encode(calendar_id);
        let ev_enc = urlencoding::encode(&google_id);
        let path = format!("/calendars/{}/events/{}", cal_enc, ev_enc);

        // Build attendees array with self responseStatus updated
        let attendees_json: Vec<serde_json::Value> = current_attendees
            .iter()
            .map(|a| {
                let status = if a.is_self == Some(true) {
                    response_status
                } else {
                    a.response_status
                        .as_ref()
                        .map(|s| match s {
                            crate::domain::event::ResponseStatus::Accepted => "accepted",
                            crate::domain::event::ResponseStatus::Declined => "declined",
                            crate::domain::event::ResponseStatus::Tentative => "tentative",
                            crate::domain::event::ResponseStatus::NeedsAction => "needsAction",
                        })
                        .unwrap_or("needsAction")
                };
                let mut v = json!({
                    "email": a.email,
                    "responseStatus": status,
                });
                if let Some(ref name) = a.display_name {
                    v["displayName"] = json!(name);
                }
                if let Some(true) = a.organizer {
                    v["organizer"] = json!(true);
                }
                if let Some(true) = a.is_self {
                    v["self"] = json!(true);
                }
                v
            })
            .collect();

        let body = json!({ "attendees": attendees_json });
        // PATCH returns the updated event, but we ignore it here
        let _raw: serde_json::Value = self.patch_json(account_email, &path, &body).await?;
        Ok(())
    }

    pub async fn update_event(
        &self,
        account_email: &str,
        calendar_id: &str,
        composite_id: &str,
        event: &CalEvent,
    ) -> Result<CalEvent> {
        let google_id = extract_google_id(composite_id);
        let cal_enc = urlencoding::encode(calendar_id);
        let ev_enc = urlencoding::encode(&google_id);
        let path = format!("/calendars/{}/events/{}", cal_enc, ev_enc);
        let body = event_to_json(event);
        let raw: GRawEvent = self.patch_json(account_email, &path, &body).await?;
        Ok(raw_to_cal_event(&raw, account_email, calendar_id))
    }

    pub async fn delete_event(
        &self,
        account_email: &str,
        calendar_id: &str,
        composite_id: &str,
    ) -> Result<()> {
        let google_id = extract_google_id(composite_id);
        let cal_enc = urlencoding::encode(calendar_id);
        let ev_enc = urlencoding::encode(&google_id);
        let path = format!("/calendars/{}/events/{}", cal_enc, ev_enc);
        self.delete(account_email, &path).await
    }
}

fn event_to_json(event: &CalEvent) -> serde_json::Value {
    event_to_json_with_opts(event, false)
}

fn event_to_json_with_opts(event: &CalEvent, add_meet: bool) -> serde_json::Value {
    use serde_json::json;
    let mut obj = json!({
        "summary": event.summary,
        "start": time_object_to_json(&event.start),
        "end": time_object_to_json(&event.end),
    });
    if let Some(ref d) = event.description {
        obj["description"] = json!(d);
    }
    if let Some(ref l) = event.location {
        obj["location"] = json!(l);
    }
    // Serialize attendees if present
    if let Some(ref attendees) = event.attendees {
        if !attendees.is_empty() {
            let list: Vec<serde_json::Value> = attendees
                .iter()
                .map(|a| {
                    let mut v = json!({ "email": a.email });
                    if let Some(ref name) = a.display_name {
                        v["displayName"] = json!(name);
                    }
                    v
                })
                .collect();
            obj["attendees"] = json!(list);
        }
    }
    if add_meet {
        obj["conferenceData"] = json!({
            "createRequest": {
                "requestId": uuid::Uuid::new_v4().to_string(),
                "conferenceSolutionKey": { "type": "hangoutsMeet" }
            }
        });
    }
    obj
}

fn time_object_to_json(t: &TimeObject) -> serde_json::Value {
    use serde_json::json;
    let mut obj = json!({});
    if let Some(ref dt) = t.date_time {
        obj["dateTime"] = json!(dt);
    }
    if let Some(ref d) = t.date {
        obj["date"] = json!(d);
    }
    if let Some(ref tz) = t.time_zone {
        obj["timeZone"] = json!(tz);
    }
    obj
}

// ── Free/Busy ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BusyPeriod {
    pub start: String,
    pub end: String,
}

impl GcalClient {
    pub async fn query_free_busy(
        &self,
        account_email: &str,
        emails: &[String],
        time_min: &str,
        time_max: &str,
    ) -> Result<HashMap<String, Vec<BusyPeriod>>> {
        use serde_json::json;
        #[derive(Deserialize)]
        struct FBPeriod {
            start: String,
            end: String,
        }
        #[derive(Deserialize)]
        struct FBCalendar {
            busy: Option<Vec<FBPeriod>>,
        }
        #[derive(Deserialize)]
        struct FBResponse {
            calendars: Option<HashMap<String, FBCalendar>>,
        }

        let body = json!({
            "timeMin": time_min,
            "timeMax": time_max,
            "items": emails.iter().map(|e| json!({"id": e})).collect::<Vec<_>>(),
        });

        let resp: FBResponse = self.post_json(account_email, "/freeBusy", &body).await?;
        let cals = resp.calendars.unwrap_or_default();
        let mut result = HashMap::new();
        for email in emails {
            let busy = cals
                .get(email)
                .and_then(|c| c.busy.as_ref())
                .map(|list| {
                    list.iter()
                        .map(|p| BusyPeriod {
                            start: p.start.clone(),
                            end: p.end.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            result.insert(email.clone(), busy);
        }
        Ok(result)
    }
}

// ── urlencoding helper (internal) ─────────────────────────────────────────────

mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}
