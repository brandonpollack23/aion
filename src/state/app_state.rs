use chrono::{Datelike, Local, NaiveDate};
use ratatui::style::Color;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

use crate::auth::tokens::AccountInfo;
use crate::config::schema::{Config, ViewMode};
use crate::domain::event::{CalEvent, ResponseStatus};
use crate::domain::layout::{layout_day, DayLayout};
use crate::domain::time::{get_days_range, get_event_end, get_local_timezone};
use crate::state::caldav_dialog::CalDavDialogState;
use crate::state::dialog::DialogState;
use crate::state::focus::FocusContext;
use crate::state::meet_with::MeetWithState;
use crate::state::message::Message;
use crate::state::overlay::{Overlay, RecurrenceScope};
use crate::state::propose_time::ProposeTimeState;
use crate::ui::theme::AppTheme;

pub const DEFAULT_SCROLL_OFFSET: usize = 28; // 7 AM * 4 slots/hour

#[derive(Debug, Clone)]
pub struct CalendarInfo {
    pub key: String, // "accountEmail:calendarId"
    pub account_email: String,
    pub calendar_id: String,
    pub display_name: String,
    pub color: Option<Color>, // hex color from Google API, None = use palette
    pub enabled: bool,
}

pub struct AppState {
    pub config: Config,
    pub theme: AppTheme,
    pub timezone: String,

    // Events
    pub events: HashMap<String, CalEvent>,

    // Navigation
    pub view_mode: ViewMode,
    pub view_anchor_day: NaiveDate,
    pub selected_day: NaiveDate,
    pub focused_column: usize,
    pub column_count: usize,

    // Selection
    pub selected_event_id: Option<String>,
    pub focus: FocusContext,

    // Timeline
    pub shared_scroll_offset: usize,
    pub all_day_expanded: bool,

    // Calendars
    pub calendars: Vec<CalendarInfo>,
    pub calendar_sidebar_visible: bool,
    pub selected_calendar_index: usize,
    pub disabled_calendars: HashSet<String>, // keys that are toggled off
    pub calendar_sidebar_scroll: usize,
    pub calendar_sidebar_height: std::cell::Cell<u16>,

    // Overlays
    pub overlay_stack: Vec<Overlay>,
    pub recurrence_scope: RecurrenceScope,
    // If a pending action requires recurrence confirmation, store its kind here
    pub pending_confirm_is_delete: bool,
    pub pending_rsvp_status: Option<String>,

    // Command bar
    pub command_input: String,
    pub command_cursor: usize,
    pub command_selected_index: usize,
    pub command_completion_query: Option<String>,
    pub command_history: Vec<String>,

    // Search
    pub search_query: String,
    pub search_results: Vec<CalEvent>,
    pub search_selected_index: usize,

    // Messages
    pub message: Option<Message>,
    pub message_log: VecDeque<Message>,

    // Event dialog (create / edit)
    pub dialog_state: Option<DialogState>,

    // Goto dialog
    pub goto_input: String,
    pub goto_preview: Option<String>,

    // Auth
    pub is_logged_in: bool,
    pub is_syncing: bool,
    pub is_auth_loading: bool,

    // Accounts management (Phase 3)
    pub accounts: Vec<AccountInfo>,
    pub accounts_selected_index: usize,
    pub accounts_delete_confirm: bool,

    // CalDAV login dialog (Phase 4)
    pub caldav_dialog: Option<CalDavDialogState>,

    // Phase 5: Notifications overlay
    pub notifications_selected_index: usize,

    // Phase 5: MeetWith dialog
    pub meet_with: Option<MeetWithState>,

    // Phase 5: ProposeTime dialog
    pub propose_time: Option<ProposeTimeState>,

    // Phase 6: splash/loader shown until first events load
    pub show_splash: bool,
    pub splash_tick: usize,

    // Phase 6: timezone display toggle
    pub show_timezone: bool,

    // Double-key detection (for gg)
    pub last_key: Option<(crossterm::event::KeyCode, Instant)>,

    // Terminal size (updated on resize)
    pub terminal_width: u16,
    pub terminal_height: u16,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let theme = AppTheme::from_config(&config.theme);
        let tz = get_local_timezone();
        let today = Local::now().date_naive();
        let column_count = config.view.columns.min(5).max(1);
        let view_mode = config.view.view_mode.clone();

        Self {
            config,
            theme,
            timezone: tz,
            events: HashMap::new(),
            view_mode,
            view_anchor_day: today,
            selected_day: today,
            focused_column: 0,
            column_count,
            selected_event_id: None,
            focus: FocusContext::Timeline,
            shared_scroll_offset: DEFAULT_SCROLL_OFFSET,
            all_day_expanded: false,
            calendars: Vec::new(),
            calendar_sidebar_visible: false,
            selected_calendar_index: 0,
            disabled_calendars: HashSet::new(),
            calendar_sidebar_scroll: 0,
            calendar_sidebar_height: std::cell::Cell::new(0),
            overlay_stack: Vec::new(),
            recurrence_scope: RecurrenceScope::This,
            pending_confirm_is_delete: false,
            pending_rsvp_status: None,
            command_input: String::new(),
            command_cursor: 0,
            command_selected_index: 0,
            command_completion_query: None,
            command_history: Vec::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected_index: 0,
            dialog_state: None,
            goto_input: String::new(),
            goto_preview: None,
            message: None,
            message_log: VecDeque::new(),
            is_logged_in: false,
            is_syncing: false,
            is_auth_loading: false,
            accounts: Vec::new(),
            accounts_selected_index: 0,
            accounts_delete_confirm: false,
            caldav_dialog: None,
            notifications_selected_index: 0,
            meet_with: None,
            propose_time: None,
            show_splash: true,
            splash_tick: 0,
            show_timezone: false,
            last_key: None,
            terminal_width: 80,
            terminal_height: 24,
        }
    }

    pub fn load_events(&mut self, events: Vec<CalEvent>) {
        self.events = events.into_iter().map(|e| (e.id.clone(), e)).collect();
        self.rebuild_calendars();
        self.is_logged_in = !self.events.is_empty();
        self.show_splash = false;
        // Auto-select nearest event on the selected day
        self.auto_select_event();
    }

    pub fn rebuild_calendars_pub(&mut self) {
        self.rebuild_calendars();
    }

    fn rebuild_calendars(&mut self) {
        let mut seen: HashMap<String, CalendarInfo> = HashMap::new();
        for event in self.events.values() {
            let key = event.calendar_key();
            if !seen.contains_key(&key) {
                // Preserve display name and color if this calendar was already known.
                let existing = self.calendars.iter().find(|c| c.key == key);
                let display_name = existing
                    .map(|c| c.display_name.clone())
                    .unwrap_or_else(|| event.calendar_id.clone().unwrap_or_else(|| "Calendar".into()));
                let color = existing.and_then(|c| c.color);
                seen.insert(
                    key.clone(),
                    CalendarInfo {
                        key: key.clone(),
                        account_email: event.account_email.clone().unwrap_or_default(),
                        calendar_id: event.calendar_id.clone().unwrap_or_default(),
                        display_name,
                        color,
                        enabled: true,
                    },
                );
            }
        }
        // Sort by account then calendar
        let mut cals: Vec<CalendarInfo> = seen.into_values().collect();
        cals.sort_by(|a, b| a.account_email.cmp(&b.account_email).then(a.calendar_id.cmp(&b.calendar_id)));
        self.calendars = cals;
    }

    pub fn filtered_events(&self) -> Vec<&CalEvent> {
        self.events
            .values()
            .filter(|e| {
                if self.disabled_calendars.is_empty() {
                    return true;
                }
                let key = e.calendar_key();
                !self.disabled_calendars.contains(&key)
            })
            .collect()
    }

    pub fn calendar_line_offset(&self, idx: usize) -> usize {
        let mut line: usize = 0;
        let mut current_account = String::new();
        for (i, cal) in self.calendars.iter().enumerate() {
            if cal.account_email != current_account {
                if !current_account.is_empty() {
                    line += 1; // blank separator between accounts
                }
                line += 1; // account header line
                current_account = cal.account_email.clone();
            }
            if i == idx {
                return line;
            }
            line += 1; // calendar entry
        }
        line
    }

    pub fn clamp_calendar_scroll(&mut self) {
        let height = self.calendar_sidebar_height.get() as usize;
        if height == 0 {
            return;
        }
        let selected_line = self.calendar_line_offset(self.selected_calendar_index);
        if selected_line >= self.calendar_sidebar_scroll + height {
            self.calendar_sidebar_scroll = selected_line.saturating_sub(height - 1);
        }
        if selected_line < self.calendar_sidebar_scroll {
            self.calendar_sidebar_scroll = selected_line;
        }
    }

    pub fn day_layout(&self, day: NaiveDate) -> DayLayout {
        let events: Vec<CalEvent> = self.filtered_events().into_iter().cloned().collect();
        layout_day(&events, day, &self.timezone)
    }

    /// Returns the list of days shown in the days sidebar
    pub fn days_list(&self, visible_lines: usize) -> Vec<NaiveDate> {
        let half = (visible_lines / 2).max(7) as i64;
        get_days_range(self.view_anchor_day, half, half)
    }

    pub fn focused_column_day(&self) -> NaiveDate {
        if self.column_count == 1 {
            return self.selected_day;
        }
        let offset = self.focused_column as i64;
        let base = self.days_in_view()[0];
        base + chrono::Duration::days(offset)
    }

    /// The days shown in the multi-column view (starting from selected_day)
    pub fn days_in_view(&self) -> Vec<NaiveDate> {
        (0..self.column_count as i64)
            .map(|i| self.selected_day + chrono::Duration::days(i))
            .collect()
    }

    pub fn top_overlay(&self) -> Option<&Overlay> {
        self.overlay_stack.last()
    }

    pub fn push_overlay(&mut self, overlay: Overlay) {
        self.focus = overlay.kind.to_focus_context();
        self.overlay_stack.push(overlay);
    }

    pub fn pop_overlay(&mut self) {
        if let Some(top) = self.overlay_stack.pop() {
            self.focus = top
                .prev_focus
                .unwrap_or(FocusContext::Timeline);
        }
    }

    pub fn show_message(&mut self, msg: Message) {
        const MAX_LOG: usize = 100;
        if self.message_log.len() >= MAX_LOG {
            self.message_log.pop_front();
        }
        self.message_log.push_back(msg.clone());
        self.message = Some(msg);
    }

    pub fn auto_select_event(&mut self) {
        use crate::domain::time::get_now_minutes;
        let day = self.selected_day;
        let layout = self.day_layout(day);
        if layout.all_day_events.is_empty() && layout.timed_events.is_empty() {
            self.selected_event_id = None;
            return;
        }
        // If already selected something valid in this column, keep it
        if let Some(ref id) = self.selected_event_id {
            let all_events = layout.chronological_events();
            if all_events.iter().any(|e| e.id == *id) {
                return;
            }
        }
        let nearest = layout.find_nearest_event(get_now_minutes());
        self.selected_event_id = nearest.map(|e| e.id.clone());
    }

    pub fn scroll_to_selected_event(&mut self, available_height: usize) {
        use crate::domain::layout::{minutes_to_slot, TOTAL_SLOTS};
        let day = self.focused_column_day();
        let layout = self.day_layout(day);

        if let Some(ref id) = self.selected_event_id {
            if let Some(tl) = layout.get_event_layout(id) {
                let slot = minutes_to_slot(tl.start_minutes) as usize;
                let offset = slot.saturating_sub(available_height / 3);
                let max_offset = TOTAL_SLOTS as usize - available_height;
                self.shared_scroll_offset = offset.min(max_offset);
                return;
            }
        }
        // Scroll to 7 AM as default
        self.shared_scroll_offset = DEFAULT_SCROLL_OFFSET;
    }

    pub fn move_event_selection(&mut self, direction: SelectionDirection) {
        let day = self.focused_column_day();
        let layout = self.day_layout(day);
        let events = layout.chronological_events();
        if events.is_empty() {
            return;
        }

        let current_idx = self
            .selected_event_id
            .as_ref()
            .and_then(|id| events.iter().position(|e| e.id == *id));

        let new_idx = match direction {
            SelectionDirection::Next => match current_idx {
                Some(i) if i + 1 < events.len() => i + 1,
                Some(_) => events.len() - 1,
                None => 0,
            },
            SelectionDirection::Prev => match current_idx {
                Some(i) if i > 0 => i - 1,
                Some(_) => 0,
                None => 0,
            },
            SelectionDirection::First => 0,
            SelectionDirection::Last => events.len() - 1,
        };

        self.selected_event_id = events.get(new_idx).map(|e| e.id.clone());
    }

    pub fn move_day_selection(&mut self, direction: i64) {
        self.selected_day = self.selected_day + chrono::Duration::days(direction);
        self.view_anchor_day = self.selected_day;
        self.selected_event_id = None;
        self.auto_select_event();
    }

    pub fn move_column(&mut self, direction: i64) {
        if self.column_count <= 1 {
            self.move_day_selection(direction);
            return;
        }
        let new_col = self.focused_column as i64 + direction;
        if new_col < 0 || new_col >= self.column_count as i64 {
            // Move the view window
            self.selected_day = self.selected_day + chrono::Duration::days(direction);
            self.view_anchor_day = self.selected_day;
        } else {
            self.focused_column = new_col as usize;
        }
        self.selected_event_id = None;
        self.auto_select_event();
    }

    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Daily => ViewMode::Monthly,
            ViewMode::Monthly => ViewMode::Daily,
        };
        self.config.view.view_mode = self.view_mode.clone();
        let _ = crate::config::loader::save_config(&self.config);
    }

    pub fn move_month(&mut self, delta: i32) {
        let date = self.selected_day;
        let total_months = date.year() * 12 + date.month() as i32 - 1 + delta;
        let new_year = total_months / 12;
        let new_month = (total_months % 12 + 1) as u32;
        let days_in_new_month = days_in_month(new_year, new_month);
        let new_day = date.day().min(days_in_new_month);
        if let Some(new_date) = NaiveDate::from_ymd_opt(new_year, new_month, new_day) {
            self.selected_day = new_date;
            self.view_anchor_day = new_date;
        }
    }

    pub fn toggle_column_count(&mut self) {
        self.column_count = match self.column_count {
            1 => 3,
            3 => 5,
            _ => 1,
        };
        self.focused_column = 0;
        self.selected_event_id = None;
        self.auto_select_event();
    }

    pub fn jump_to_now(&mut self) {
        self.selected_day = Local::now().date_naive();
        self.view_anchor_day = self.selected_day;
        self.focused_column = 0;
        self.auto_select_event();
    }

    pub fn search_events(&mut self, query: &str) {
        self.search_query = query.to_string();
        let q = query.to_lowercase();
        if q.is_empty() {
            self.search_results = Vec::new();
            return;
        }
        let mut results: Vec<CalEvent> = self
            .events
            .values()
            .filter(|e| {
                e.summary.to_lowercase().contains(&q)
                    || e.description
                        .as_deref()
                        .map_or(false, |d| d.to_lowercase().contains(&q))
                    || e.location
                        .as_deref()
                        .map_or(false, |l| l.to_lowercase().contains(&q))
            })
            .cloned()
            .collect();

        results.sort_by(|a, b| {
            let a_dt = a.start.date_time.as_deref().or(a.start.date.as_deref()).unwrap_or("");
            let b_dt = b.start.date_time.as_deref().or(b.start.date.as_deref()).unwrap_or("");
            a_dt.cmp(b_dt)
        });
        self.search_results = results;
        self.search_selected_index = 0;
    }

    /// Reload accounts list from disk (called after auth events).
    pub fn reload_accounts(&mut self) {
        self.accounts = crate::auth::tokens::get_accounts()
            .into_iter()
            .map(|d| d.account)
            .collect();
        self.accounts.sort_by(|a, b| a.email.cmp(&b.email));
        self.accounts_selected_index = self
            .accounts_selected_index
            .min(self.accounts.len().saturating_sub(1));
    }

    /// Events where self has not yet responded (RSVP = needsAction).
    pub fn pending_invites(&self) -> Vec<&CalEvent> {
        let now_utc = chrono::Utc::now().naive_utc();
        let mut invites: Vec<&CalEvent> = self
            .events
            .values()
            .filter(|e| {
                e.self_response_status()
                    .map_or(false, |s| *s == ResponseStatus::NeedsAction)
            })
            .filter(|e| {
                self.disabled_calendars.is_empty()
                    || !self.disabled_calendars.contains(&e.calendar_key())
            })
            .filter(|e| {
                get_event_end(e, &self.timezone)
                    .map_or(true, |end| end.naive_utc() >= now_utc)
            })
            .collect();
        invites.sort_by(|a, b| {
            let a_start = a.start.date_time.as_deref().or(a.start.date.as_deref()).unwrap_or("");
            let b_start = b.start.date_time.as_deref().or(b.start.date.as_deref()).unwrap_or("");
            a_start.cmp(b_start)
        });
        invites
    }

    /// Apply calendar list from Google API: update display names and colors.
    pub fn apply_api_calendars(&mut self, api_cals: &[crate::api::gcal::CalendarListEntry]) {
        for api_cal in api_cals {
            let key = format!("{}:{}", api_cal.account_email, api_cal.id);
            if let Some(cal) = self.calendars.iter_mut().find(|c| c.key == key) {
                cal.display_name = api_cal.summary.clone();
                if let Some(ref bg) = api_cal.background_color {
                    cal.color = parse_hex_color(bg);
                }
            } else {
                self.calendars.push(CalendarInfo {
                    key: key.clone(),
                    account_email: api_cal.account_email.clone(),
                    calendar_id: api_cal.id.clone(),
                    display_name: api_cal.summary.clone(),
                    color: api_cal.background_color.as_deref().and_then(parse_hex_color),
                    enabled: true,
                });
            }
        }
        self.calendars.sort_by(|a, b| {
            a.account_email.cmp(&b.account_email).then(a.calendar_id.cmp(&b.calendar_id))
        });
    }
}

pub fn parse_hex_color(s: &str) -> Option<ratatui::style::Color> {
    if s.starts_with('#') && s.len() == 7 {
        let r = u8::from_str_radix(&s[1..3], 16).ok()?;
        let g = u8::from_str_radix(&s[3..5], 16).ok()?;
        let b = u8::from_str_radix(&s[5..7], 16).ok()?;
        return Some(ratatui::style::Color::Rgb(r, g, b));
    }
    None
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(28)
}

pub enum SelectionDirection {
    Next,
    Prev,
    First,
    Last,
}
