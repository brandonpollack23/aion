use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_text_primary() -> String {
    "white".to_string()
}
fn default_text_secondary() -> String {
    "whiteBright".to_string()
}
fn default_text_dim() -> String {
    "blackBright".to_string()
}
fn default_text_weekend() -> String {
    "redBright".to_string()
}
fn default_accent_primary() -> String {
    "cyan".to_string()
}
fn default_accent_success() -> String {
    "green".to_string()
}
fn default_accent_warning() -> String {
    "yellow".to_string()
}
fn default_accent_error() -> String {
    "red".to_string()
}
fn default_event_default() -> String {
    "cyan".to_string()
}
fn default_event_ooo() -> String {
    "magenta".to_string()
}
fn default_event_focus() -> String {
    "blue".to_string()
}
fn default_event_birthday() -> String {
    "yellow".to_string()
}
fn default_selection_bg() -> String {
    "blackBright".to_string()
}
fn default_selection_text() -> String {
    "whiteBright".to_string()
}
fn default_status_accepted() -> String {
    "green".to_string()
}
fn default_status_declined() -> String {
    "red".to_string()
}
fn default_status_tentative() -> String {
    "yellow".to_string()
}
fn default_status_needs_action() -> String {
    "blackBright".to_string()
}
fn default_modal_bg() -> String {
    "black".to_string()
}
fn default_modal_border() -> String {
    "blackBright".to_string()
}
fn default_statusbar_bg() -> String {
    "black".to_string()
}
fn default_statusbar_text() -> String {
    "white".to_string()
}
fn default_timezone_local_bg() -> String {
    "darkGray".to_string()
}
fn default_input_bg() -> String {
    "black".to_string()
}
fn default_input_text() -> String {
    "white".to_string()
}
fn default_input_placeholder() -> String {
    "blackBright".to_string()
}
fn default_view_columns() -> usize {
    1
}
fn default_view_mode() -> ViewMode {
    ViewMode::Daily
}
fn default_additional_timezones() -> Vec<String> {
    vec![]
}
fn default_caldav() -> Vec<CalDAVAccount> {
    vec![]
}
fn default_calendar_colors() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("1".to_string(), "blue".to_string());
    m.insert("2".to_string(), "green".to_string());
    m.insert("3".to_string(), "magenta".to_string());
    m.insert("4".to_string(), "red".to_string());
    m.insert("5".to_string(), "yellow".to_string());
    m.insert("6".to_string(), "cyan".to_string());
    m
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeText {
    #[serde(default = "default_text_primary")]
    pub primary: String,
    #[serde(default = "default_text_secondary")]
    pub secondary: String,
    #[serde(default = "default_text_dim")]
    pub dim: String,
    #[serde(default = "default_text_weekend")]
    pub weekend: String,
}

impl Default for ThemeText {
    fn default() -> Self {
        Self {
            primary: default_text_primary(),
            secondary: default_text_secondary(),
            dim: default_text_dim(),
            weekend: default_text_weekend(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeAccent {
    #[serde(default = "default_accent_primary")]
    pub primary: String,
    #[serde(default = "default_accent_success")]
    pub success: String,
    #[serde(default = "default_accent_warning")]
    pub warning: String,
    #[serde(default = "default_accent_error")]
    pub error: String,
}

impl Default for ThemeAccent {
    fn default() -> Self {
        Self {
            primary: default_accent_primary(),
            success: default_accent_success(),
            warning: default_accent_warning(),
            error: default_accent_error(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeEventType {
    #[serde(default = "default_event_default")]
    pub default: String,
    #[serde(rename = "outOfOffice", default = "default_event_ooo")]
    pub out_of_office: String,
    #[serde(rename = "focusTime", default = "default_event_focus")]
    pub focus_time: String,
    #[serde(default = "default_event_birthday")]
    pub birthday: String,
}

impl Default for ThemeEventType {
    fn default() -> Self {
        Self {
            default: default_event_default(),
            out_of_office: default_event_ooo(),
            focus_time: default_event_focus(),
            birthday: default_event_birthday(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSelection {
    #[serde(default = "default_selection_bg")]
    pub background: String,
    #[serde(default = "default_selection_text")]
    pub text: String,
}

impl Default for ThemeSelection {
    fn default() -> Self {
        Self {
            background: default_selection_bg(),
            text: default_selection_text(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeStatus {
    #[serde(default = "default_status_accepted")]
    pub accepted: String,
    #[serde(default = "default_status_declined")]
    pub declined: String,
    #[serde(default = "default_status_tentative")]
    pub tentative: String,
    #[serde(rename = "needsAction", default = "default_status_needs_action")]
    pub needs_action: String,
}

impl Default for ThemeStatus {
    fn default() -> Self {
        Self {
            accepted: default_status_accepted(),
            declined: default_status_declined(),
            tentative: default_status_tentative(),
            needs_action: default_status_needs_action(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeModal {
    #[serde(default = "default_modal_bg")]
    pub background: String,
    #[serde(default = "default_modal_border")]
    pub border: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeInput {
    #[serde(default = "default_input_bg")]
    pub background: String,
    #[serde(default = "default_input_text")]
    pub text: String,
    #[serde(default = "default_input_placeholder")]
    pub placeholder: String,
}

impl Default for ThemeInput {
    fn default() -> Self {
        Self {
            background: default_input_bg(),
            text: default_input_text(),
            placeholder: default_input_placeholder(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeStatusBar {
    #[serde(default = "default_statusbar_bg")]
    pub background: String,
    #[serde(default = "default_statusbar_text")]
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Theme {
    #[serde(default)]
    pub text: ThemeText,
    #[serde(default)]
    pub accent: ThemeAccent,
    #[serde(rename = "eventType", default)]
    pub event_type: ThemeEventType,
    #[serde(default)]
    pub selection: ThemeSelection,
    #[serde(default)]
    pub status: ThemeStatus,
    #[serde(default)]
    pub modal: ThemeModal,
    #[serde(default)]
    pub input: ThemeInput,
    #[serde(rename = "statusBar", default)]
    pub status_bar: ThemeStatusBar,
    #[serde(rename = "calendarColors", default = "default_calendar_colors")]
    pub calendar_colors: HashMap<String, String>,
    #[serde(rename = "timezoneLocalBg", default = "default_timezone_local_bg")]
    pub timezone_local_bg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoogleConfig {
    #[serde(rename = "clientId")]
    pub client_id: Option<String>,
    #[serde(rename = "clientSecret")]
    pub client_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    #[default]
    Daily,
    Monthly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewConfig {
    #[serde(default = "default_view_columns")]
    pub columns: usize,
    #[serde(default = "default_view_mode")]
    pub view_mode: ViewMode,
    #[serde(
        rename = "additionalTimezones",
        default = "default_additional_timezones"
    )]
    pub additional_timezones: Vec<String>,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            columns: 1,
            view_mode: ViewMode::Daily,
            additional_timezones: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalDAVAccount {
    pub name: String,
    pub email: String,
    pub server_url: String,
    pub username: String,
    pub password: Option<String>,
    pub password_command: Option<String>,
}

fn default_hidden_calendars() -> std::collections::HashSet<String> {
    std::collections::HashSet::new()
}

fn default_notifications_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsConfig {
    #[serde(default = "default_notifications_enabled")]
    pub enabled: bool,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub google: GoogleConfig,
    #[serde(default)]
    pub view: ViewConfig,
    #[serde(default = "default_caldav")]
    pub caldav: Vec<CalDAVAccount>,
    #[serde(default)]
    pub notifications: NotificationsConfig,
    #[serde(rename = "defaultCalendar")]
    pub default_calendar: Option<String>,
    #[serde(rename = "hiddenCalendars", default = "default_hidden_calendars")]
    pub hidden_calendars: std::collections::HashSet<String>,
}
