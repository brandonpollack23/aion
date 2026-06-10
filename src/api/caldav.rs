use anyhow::{Context, Result};
use reqwest::{header, Client, Method, StatusCode};

use crate::api::gcal::CalendarListEntry;
use crate::api::ical::{
    extract_uid, extract_uid_from_composite, generate_icalendar, make_composite_id, parse_icalendar,
};
use crate::config::schema::CalDAVAccount;
use crate::domain::event::CalEvent;

const CALDAV_NS: &str = "urn:ietf:params:xml:ns:caldav";
const DAV_NS: &str = "DAV:";
const CS_NS: &str = "http://calendarserver.org/ns/";
const ICAL_NS: &str = "http://apple.com/ns/ical/";

// ── CalDAV presets ─────────────────────────────────────────────────────────────

pub struct CalDavPreset {
    pub name: &'static str,
    pub server_url: &'static str,
    pub help: &'static str,
}

pub const CALDAV_PRESETS: &[CalDavPreset] = &[
    CalDavPreset {
        name: "iCloud",
        server_url: "https://caldav.icloud.com",
        help: "Use your Apple ID email and an app-specific password from appleid.apple.com",
    },
    CalDavPreset {
        name: "Fastmail",
        server_url: "https://caldav.fastmail.com",
        help: "Use your Fastmail email and an app-specific password",
    },
    CalDavPreset {
        name: "Nextcloud",
        server_url: "",
        help: "Use https://your-server.com/remote.php/dav and your Nextcloud credentials",
    },
    CalDavPreset {
        name: "Radicale",
        server_url: "",
        help: "Use http://your-server:5232 and your Radicale credentials",
    },
    CalDavPreset {
        name: "Baïkal",
        server_url: "",
        help: "Use https://your-server.com/dav.php and your Baïkal credentials",
    },
    CalDavPreset {
        name: "Custom",
        server_url: "",
        help: "Enter the CalDAV server URL manually",
    },
];

// ── Result types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CalDavCalendar {
    pub url: String,
    pub display_name: String,
    pub color: Option<String>,
    pub ctag: Option<String>,
    pub supported_components: Vec<String>,
}

#[derive(Debug)]
pub struct CalDavObject {
    pub url: String,
    pub etag: Option<String>,
    pub ical_data: String,
}

pub struct IncrementalSyncResult {
    pub events: Vec<CalEvent>,
    pub deleted: Vec<String>,
    pub ctag: Option<String>,
    pub full_sync: bool,
}

// ── Client ─────────────────────────────────────────────────────────────────────

pub struct CalDavClient {
    http: Client,
    base_url: String,
    username: String,
    password: String,
}

impl CalDavClient {
    pub fn new(base_url: &str, username: &str, password: &str) -> Self {
        Self {
            http: Client::builder()
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .unwrap_or_default(),
            base_url: base_url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    // ── Discovery ──────────────────────────────────────────────────────────────

    /// PROPFIND the server URL to find the current-user-principal
    async fn discover_principal(&self) -> Result<String> {
        let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:current-user-principal/>
    <D:principal-URL/>
  </D:prop>
</D:propfind>"#;

        let resp = self.propfind(&self.base_url, body, "0").await?;

        // Try /.well-known/caldav if the server didn't return a principal
        let principal = extract_href_from_propfind(&resp, "current-user-principal")
            .or_else(|| extract_href_from_propfind(&resp, "principal-URL"));

        if let Some(p) = principal {
            return Ok(self.resolve_url(&p));
        }

        // Many servers: the provided URL IS the principal or home
        // Try /.well-known/caldav redirect
        let well_known = format!("{}/.well-known/caldav", self.base_url);
        if let Ok(wk_resp) = self.propfind(&well_known, body, "0").await {
            if let Some(p) = extract_href_from_propfind(&wk_resp, "current-user-principal") {
                return Ok(self.resolve_url(&p));
            }
        }

        // Fallback: assume base_url is the principal
        Ok(self.base_url.clone())
    }

    /// PROPFIND the principal URL to find the calendar-home-set
    async fn discover_calendar_home(&self, principal_url: &str) -> Result<String> {
        let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<D:propfind xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop>
    <C:calendar-home-set/>
  </D:prop>
</D:propfind>"#;

        let resp = self.propfind(principal_url, body, "0").await?;

        if let Some(href) = extract_href_from_propfind(&resp, "calendar-home-set") {
            return Ok(self.resolve_url(&href));
        }

        // Fallback: principal URL is also the home
        Ok(principal_url.to_string())
    }

    pub async fn get_calendar_home(&self) -> Result<String> {
        let principal = self.discover_principal().await?;
        self.discover_calendar_home(&principal).await
    }

    // ── Calendar listing ───────────────────────────────────────────────────────

    pub async fn fetch_calendars(&self) -> Result<Vec<CalDavCalendar>> {
        let home = self.get_calendar_home().await?;
        self.fetch_calendars_from_home(&home).await
    }

    pub async fn fetch_calendars_from_home(&self, home_url: &str) -> Result<Vec<CalDavCalendar>> {
        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<D:propfind xmlns:D="{DAV}" xmlns:C="{CALDAV}" xmlns:CS="{CS}" xmlns:I="{ICAL}">
  <D:prop>
    <D:displayname/>
    <D:resourcetype/>
    <CS:getctag/>
    <C:supported-calendar-component-set/>
    <I:calendar-color/>
  </D:prop>
</D:propfind>"#,
            DAV = DAV_NS,
            CALDAV = CALDAV_NS,
            CS = CS_NS,
            ICAL = ICAL_NS,
        );

        let resp = self.propfind(home_url, &body, "1").await?;
        Ok(parse_calendar_list(&resp, home_url))
    }

    pub async fn get_ctag(&self, calendar_url: &str) -> Result<Option<String>> {
        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<D:propfind xmlns:D="{DAV}" xmlns:CS="{CS}">
  <D:prop>
    <CS:getctag/>
  </D:prop>
</D:propfind>"#,
            DAV = DAV_NS,
            CS = CS_NS,
        );
        let resp = self.propfind(calendar_url, &body, "0").await?;
        Ok(extract_text_from_xml(&resp, "getctag"))
    }

    // ── Event fetching ─────────────────────────────────────────────────────────

    pub async fn fetch_events_in_range(
        &self,
        calendar_url: &str,
        time_min: &str,
        time_max: &str,
    ) -> Result<Vec<CalDavObject>> {
        // Convert ISO times to iCal format (strip separators, ensure Z suffix)
        let ical_min = iso_to_ical_time(time_min);
        let ical_max = iso_to_ical_time(time_max);

        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<C:calendar-query xmlns:D="{DAV}" xmlns:C="{CALDAV}">
  <D:prop>
    <D:getetag/>
    <C:calendar-data/>
  </D:prop>
  <C:filter>
    <C:comp-filter name="VCALENDAR">
      <C:comp-filter name="VEVENT">
        <C:time-range start="{MIN}" end="{MAX}"/>
      </C:comp-filter>
    </C:comp-filter>
  </C:filter>
</C:calendar-query>"#,
            DAV = DAV_NS,
            CALDAV = CALDAV_NS,
            MIN = ical_min,
            MAX = ical_max,
        );

        let resp = self.report(calendar_url, &body).await?;
        Ok(parse_calendar_objects(&resp, calendar_url, self))
    }

    // ── CRUD ───────────────────────────────────────────────────────────────────

    pub async fn create_event(
        &self,
        calendar_url: &str,
        uid: &str,
        ical_data: &str,
    ) -> Result<String> {
        let event_url = format!("{}/{}.ics", calendar_url.trim_end_matches('/'), uid);
        let resp = self
            .http
            .put(&event_url)
            .basic_auth(&self.username, Some(&self.password))
            .header(header::CONTENT_TYPE, "text/calendar; charset=utf-8")
            .header("If-None-Match", "*")
            .body(ical_data.to_string())
            .send()
            .await
            .context("CalDAV PUT (create) request failed")?;

        if !resp.status().is_success() && resp.status() != StatusCode::NO_CONTENT {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("CalDAV PUT failed: {} — {}", status, text);
        }
        Ok(event_url)
    }

    pub async fn update_event(
        &self,
        event_url: &str,
        ical_data: &str,
        etag: Option<&str>,
    ) -> Result<()> {
        let mut req = self
            .http
            .put(event_url)
            .basic_auth(&self.username, Some(&self.password))
            .header(header::CONTENT_TYPE, "text/calendar; charset=utf-8")
            .body(ical_data.to_string());

        if let Some(e) = etag {
            req = req.header("If-Match", e);
        }

        let resp = req
            .send()
            .await
            .context("CalDAV PUT (update) request failed")?;
        if !resp.status().is_success() && resp.status() != StatusCode::NO_CONTENT {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("CalDAV PUT (update) failed: {} — {}", status, text);
        }
        Ok(())
    }

    pub async fn delete_event(&self, event_url: &str, etag: Option<&str>) -> Result<()> {
        let mut req = self
            .http
            .delete(event_url)
            .basic_auth(&self.username, Some(&self.password));
        if let Some(e) = etag {
            req = req.header("If-Match", e);
        }
        let resp = req.send().await.context("CalDAV DELETE request failed")?;
        if !resp.status().is_success()
            && resp.status() != StatusCode::NO_CONTENT
            && resp.status() != StatusCode::NOT_FOUND
        {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("CalDAV DELETE failed: {} — {}", status, text);
        }
        Ok(())
    }

    // ── Sync ───────────────────────────────────────────────────────────────────

    pub async fn sync_calendar(
        &self,
        calendar_url: &str,
        account_email: &str,
        time_min: &str,
        time_max: &str,
        stored_ctag: Option<&str>,
    ) -> Result<IncrementalSyncResult> {
        let current_ctag = self.get_ctag(calendar_url).await.ok().flatten();

        // If ctags match, nothing changed
        if stored_ctag.is_some() && stored_ctag == current_ctag.as_deref() {
            return Ok(IncrementalSyncResult {
                events: vec![],
                deleted: vec![],
                ctag: current_ctag,
                full_sync: false,
            });
        }

        // Refetch all events in range
        let objects = self
            .fetch_events_in_range(calendar_url, time_min, time_max)
            .await?;

        let events: Vec<CalEvent> = objects
            .iter()
            .flat_map(|obj| {
                parse_icalendar(&obj.ical_data, Some(account_email), Some(calendar_url))
            })
            .collect();

        Ok(IncrementalSyncResult {
            events,
            deleted: vec![],
            ctag: current_ctag,
            full_sync: true,
        })
    }

    // ── Test connection ────────────────────────────────────────────────────────

    pub async fn test_connection(&self) -> Result<String> {
        let cals = self.fetch_calendars().await?;
        Ok(format!("{} calendar(s) found", cals.len()))
    }

    // ── Find event URL by UID ──────────────────────────────────────────────────

    pub async fn find_event_url_by_uid(
        &self,
        calendar_url: &str,
        uid: &str,
    ) -> Result<Option<(String, Option<String>)>> {
        // Fetch all objects from the calendar and search by UID
        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<C:calendar-query xmlns:D="{DAV}" xmlns:C="{CALDAV}">
  <D:prop>
    <D:getetag/>
    <C:calendar-data/>
  </D:prop>
  <C:filter>
    <C:comp-filter name="VCALENDAR">
      <C:comp-filter name="VEVENT"/>
    </C:comp-filter>
  </C:filter>
</C:calendar-query>"#,
            DAV = DAV_NS,
            CALDAV = CALDAV_NS,
        );
        let resp = self.report(calendar_url, &body).await?;
        let objects = parse_calendar_objects(&resp, calendar_url, self);
        for obj in objects {
            if let Some(found_uid) = extract_uid(&obj.ical_data) {
                if found_uid == uid {
                    return Ok(Some((obj.url, obj.etag)));
                }
            }
        }
        Ok(None)
    }

    // ── HTTP helpers ───────────────────────────────────────────────────────────

    async fn propfind(&self, url: &str, body: &str, depth: &str) -> Result<String> {
        let resp = self
            .http
            .request(Method::from_bytes(b"PROPFIND").unwrap(), url)
            .basic_auth(&self.username, Some(&self.password))
            .header(header::CONTENT_TYPE, "application/xml; charset=utf-8")
            .header("Depth", depth)
            .body(body.to_string())
            .send()
            .await
            .with_context(|| format!("PROPFIND {} failed", url))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("PROPFIND {} returned {}: {}", url, status, text);
        }
        resp.text()
            .await
            .context("Failed to read PROPFIND response")
    }

    async fn report(&self, url: &str, body: &str) -> Result<String> {
        let resp = self
            .http
            .request(Method::from_bytes(b"REPORT").unwrap(), url)
            .basic_auth(&self.username, Some(&self.password))
            .header(header::CONTENT_TYPE, "application/xml; charset=utf-8")
            .header("Depth", "1")
            .body(body.to_string())
            .send()
            .await
            .with_context(|| format!("REPORT {} failed", url))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("REPORT {} returned {}: {}", url, status, text);
        }
        resp.text().await.context("Failed to read REPORT response")
    }

    fn resolve_url(&self, href: &str) -> String {
        if href.starts_with("http://") || href.starts_with("https://") {
            return href.to_string();
        }
        // Parse base to get origin
        if let Ok(base) = url::Url::parse(&self.base_url) {
            let origin = format!("{}://{}", base.scheme(), base.host_str().unwrap_or(""));
            let port = base.port().map(|p| format!(":{}", p)).unwrap_or_default();
            return format!("{}{}{}", origin, port, href);
        }
        href.to_string()
    }
}

// ── High-level convenience functions ──────────────────────────────────────────

pub async fn resolve_caldav_password(account: &CalDAVAccount) -> Result<String> {
    crate::auth::password::resolve_password(
        account.password.as_deref(),
        account.password_command.as_deref(),
        Some(&account.email),
    )
    .await
}

pub async fn get_caldav_calendars(account: &CalDAVAccount) -> Result<Vec<CalendarListEntry>> {
    let password = resolve_caldav_password(account).await?;
    let client = CalDavClient::new(&account.server_url, &account.username, &password);
    let cals = client.fetch_calendars().await?;

    Ok(cals
        .into_iter()
        .filter(|c| {
            c.supported_components.is_empty()
                || c.supported_components.iter().any(|s| s == "VEVENT")
        })
        .enumerate()
        .map(|(i, c)| CalendarListEntry {
            id: c.url.clone(),
            summary: c.display_name.clone(),
            background_color: c.color,
            foreground_color: None,
            primary: Some(i == 0),
            access_role: "owner".to_string(),
            account_email: account.email.clone(),
        })
        .collect())
}

pub async fn sync_caldav_calendar(
    account: &CalDAVAccount,
    calendar_url: &str,
    time_min: &str,
    time_max: &str,
    stored_ctag: Option<&str>,
) -> Result<IncrementalSyncResult> {
    let password = resolve_caldav_password(account).await?;
    let client = CalDavClient::new(&account.server_url, &account.username, &password);
    client
        .sync_calendar(
            calendar_url,
            &account.email,
            time_min,
            time_max,
            stored_ctag,
        )
        .await
}

pub async fn create_caldav_event(
    account: &CalDAVAccount,
    calendar_url: &str,
    event: &CalEvent,
) -> Result<CalEvent> {
    let password = resolve_caldav_password(account).await?;
    let client = CalDavClient::new(&account.server_url, &account.username, &password);

    let uid = uuid::Uuid::new_v4().to_string();
    let ical = generate_icalendar(event, Some(&uid));
    client.create_event(calendar_url, &uid, &ical).await?;

    let mut created = event.clone();
    created.id = make_composite_id(Some(&account.email), Some(calendar_url), &uid);
    created.account_email = Some(account.email.clone());
    created.calendar_id = Some(calendar_url.to_string());
    Ok(created)
}

pub async fn update_caldav_event(
    account: &CalDAVAccount,
    calendar_url: &str,
    event: &CalEvent,
) -> Result<()> {
    let password = resolve_caldav_password(account).await?;
    let client = CalDavClient::new(&account.server_url, &account.username, &password);

    let uid = extract_uid_from_composite(&event.id, &account.email, calendar_url);
    let ical = generate_icalendar(event, Some(&uid));

    let found = client.find_event_url_by_uid(calendar_url, &uid).await?;
    match found {
        Some((url, etag)) => client.update_event(&url, &ical, etag.as_deref()).await,
        None => {
            // Not found — create instead
            client.create_event(calendar_url, &uid, &ical).await?;
            Ok(())
        }
    }
}

pub async fn delete_caldav_event(
    account: &CalDAVAccount,
    calendar_url: &str,
    event_id: &str,
) -> Result<()> {
    let password = resolve_caldav_password(account).await?;
    let client = CalDavClient::new(&account.server_url, &account.username, &password);

    let uid = extract_uid_from_composite(event_id, &account.email, calendar_url);
    let found = client.find_event_url_by_uid(calendar_url, &uid).await?;
    if let Some((url, etag)) = found {
        client.delete_event(&url, etag.as_deref()).await?;
    }
    Ok(())
}

pub async fn test_caldav_connection(account: &CalDAVAccount) -> Result<String> {
    let password = resolve_caldav_password(account).await?;
    let client = CalDavClient::new(&account.server_url, &account.username, &password);
    client.test_connection().await
}

// ── XML parsing helpers ────────────────────────────────────────────────────────

fn extract_href_from_propfind(xml: &str, property: &str) -> Option<String> {
    // Simple string-based extraction for common patterns
    // Looks for <property><href>...</href></property>
    let prop_lower = property.to_lowercase();

    // Try to find the property in XML (handles various namespace prefixes)
    let doc = roxmltree::Document::parse(xml).ok()?;
    for node in doc.descendants() {
        let name = node.tag_name().name().to_lowercase();
        if name == prop_lower {
            // Look for href child
            for child in node.children() {
                if child.tag_name().name().to_lowercase() == "href" {
                    return child.text().map(|s| s.trim().to_string());
                }
            }
        }
    }
    None
}

fn extract_text_from_xml(xml: &str, property: &str) -> Option<String> {
    let prop_lower = property.to_lowercase();
    let doc = roxmltree::Document::parse(xml).ok()?;
    for node in doc.descendants() {
        if node.tag_name().name().to_lowercase() == prop_lower {
            return node.text().map(|s| s.trim().to_string());
        }
    }
    None
}

fn parse_calendar_list(xml: &str, home_url: &str) -> Vec<CalDavCalendar> {
    let doc = match roxmltree::Document::parse(xml) {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut calendars = Vec::new();

    for response in doc
        .descendants()
        .filter(|n| n.tag_name().name().to_lowercase() == "response")
    {
        // Check that this is a calendar collection (resourcetype contains calendar)
        let is_calendar = response
            .descendants()
            .any(|n| n.tag_name().name().to_lowercase() == "calendar");

        if !is_calendar {
            continue;
        }

        // Get href
        let href = response
            .children()
            .find(|n| n.tag_name().name().to_lowercase() == "href")
            .and_then(|n| n.text())
            .map(|s| s.trim().to_string());

        let href = match href {
            Some(h) if !h.ends_with('/') || h.len() > 1 => h,
            _ => continue,
        };

        // Skip if this is the home collection itself
        let home_path = url::Url::parse(home_url)
            .map(|u| u.path().trim_end_matches('/').to_string())
            .unwrap_or_default();
        let href_path = href.trim_end_matches('/');
        if href_path == home_path {
            continue;
        }

        let display_name = response
            .descendants()
            .find(|n| n.tag_name().name().to_lowercase() == "displayname")
            .and_then(|n| n.text())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| {
                href.trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or("Calendar")
                    .to_string()
            });

        let ctag = response
            .descendants()
            .find(|n| n.tag_name().name().to_lowercase() == "getctag")
            .and_then(|n| n.text())
            .map(|s| s.trim().to_string());

        let color = response
            .descendants()
            .find(|n| n.tag_name().name().to_lowercase() == "calendar-color")
            .and_then(|n| n.text())
            .map(|s| {
                let s = s.trim();
                if s.starts_with('#') {
                    s[..7.min(s.len())].to_string()
                } else {
                    s.to_string()
                }
            });

        let supported_components: Vec<String> = response
            .descendants()
            .filter(|n| n.tag_name().name().to_lowercase() == "comp")
            .filter_map(|n| n.attribute("name").map(|s| s.to_string()))
            .collect();

        calendars.push(CalDavCalendar {
            url: href,
            display_name,
            color,
            ctag,
            supported_components,
        });
    }

    calendars
}

fn parse_calendar_objects(
    xml: &str,
    calendar_url: &str,
    client: &CalDavClient,
) -> Vec<CalDavObject> {
    let doc = match roxmltree::Document::parse(xml) {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut objects = Vec::new();

    for response in doc
        .descendants()
        .filter(|n| n.tag_name().name().to_lowercase() == "response")
    {
        let href = match response
            .children()
            .find(|n| n.tag_name().name().to_lowercase() == "href")
            .and_then(|n| n.text())
            .map(|s| s.trim().to_string())
        {
            Some(h) => h,
            None => continue,
        };

        // Skip the calendar collection itself
        if href.ends_with('/') && !href.contains(".ics") {
            continue;
        }

        let etag = response
            .descendants()
            .find(|n| n.tag_name().name().to_lowercase() == "getetag")
            .and_then(|n| n.text())
            .map(|s| s.trim().trim_matches('"').to_string());

        let ical_data = response
            .descendants()
            .find(|n| n.tag_name().name().to_lowercase() == "calendar-data")
            .and_then(|n| n.text())
            .map(|s| s.to_string())
            .unwrap_or_default();

        if ical_data.is_empty() {
            continue;
        }

        let url = client.resolve_url(&href);
        objects.push(CalDavObject {
            url,
            etag,
            ical_data,
        });
    }

    objects
}

fn iso_to_ical_time(iso: &str) -> String {
    // "2024-01-15T00:00:00Z" → "20240115T000000Z"
    iso.replace('-', "")
        .replace(':', "")
        .replace('.', "")
        .split('+')
        .next()
        .unwrap_or(iso)
        .to_string()
        .replace("000Z", "Z")
        .replace(' ', "T")
}
