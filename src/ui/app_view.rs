use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::state::app_state::AppState;
use crate::state::focus::FocusContext;
use crate::state::overlay::OverlayKind;
use crate::ui::command_bar::{render_command_completions, render_command_input};
use crate::config::schema::ViewMode;
use crate::ui::day_view::render_day_view;
use crate::ui::month_view::render_month_view;
use crate::ui::header::render_header;
use crate::ui::overlays::accounts_dialog::render_accounts_dialog;
use crate::ui::overlays::caldav_login_dialog::render_caldav_login_dialog;
use crate::ui::overlays::confirm_modal::render_confirm_modal;
use crate::ui::overlays::details_panel::render_details_panel;
use crate::ui::overlays::event_dialog::render_event_dialog;
use crate::ui::overlays::goto_dialog::render_goto_dialog;
use crate::ui::overlays::help_dialog::render_help_dialog;
use crate::ui::overlays::meet_with_dialog::render_meet_with_dialog;
use crate::ui::overlays::messages_dialog::render_messages_dialog;
use crate::ui::overlays::notifications_panel::render_notifications_panel;
use crate::ui::overlays::propose_time_dialog::render_propose_time_dialog;
use crate::ui::overlays::login_prompt::render_login_prompt;
use crate::ui::overlays::splash::render_splash;
use crate::ui::search_view::render_search_view;
use crate::ui::status_bar::render_status_bar;

pub fn render_app(app: &AppState, frame: &mut Frame) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Fill(1),   // main view
            Constraint::Length(1), // status bar / command input
        ])
        .split(area);

    render_header(app, frame, chunks[0]);
    match app.view_mode {
        ViewMode::Daily => render_day_view(app, frame, chunks[1]),
        ViewMode::Monthly => render_month_view(app, frame, chunks[1]),
    };

    // Status bar or command input
    if app.focus == FocusContext::Command {
        render_command_input(app, frame, chunks[2]);
        render_command_completions(app, frame, area);
    } else {
        render_status_bar(app, frame, chunks[2]);
    }

    // Search overlay (inline, no overlay_stack entry)
    if app.focus == FocusContext::Search {
        render_search_view(app, frame, chunks[1]);
    }

    // Splash shown while initial events are loading
    if app.show_splash {
        render_splash(app, frame, area);
        return;
    }

    // Overlay stack — rendered in push order so topmost is drawn last
    for overlay in &app.overlay_stack {
        match overlay.kind {
            OverlayKind::Details => render_details_panel(app, frame, area),
            OverlayKind::Help => render_help_dialog(app, frame, area),
            OverlayKind::Confirm => {
                render_confirm_modal(app, frame, area, app.pending_confirm_is_delete)
            }
            OverlayKind::Dialog => render_event_dialog(app, frame, area),
            OverlayKind::Goto => render_goto_dialog(app, frame, area),
            OverlayKind::Messages => render_messages_dialog(app, frame, area),
            OverlayKind::LoginPrompt => render_login_prompt(app, frame, area),
            OverlayKind::Accounts => render_accounts_dialog(app, frame, area),
            OverlayKind::CalDavLogin => render_caldav_login_dialog(app, frame, area),
            OverlayKind::Notifications => render_notifications_panel(app, frame, area),
            OverlayKind::ProposeTime => render_propose_time_dialog(app, frame, area),
            OverlayKind::MeetWith => render_meet_with_dialog(app, frame, area),
        }
    }
}
