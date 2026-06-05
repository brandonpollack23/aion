use crate::state::focus::FocusContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecurrenceScope {
    This,
    Following,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayKind {
    Details,
    Dialog,
    Confirm,
    Help,
    Notifications,
    ProposeTime,
    Goto,
    MeetWith,
    Accounts,
    CalDavLogin,
    Messages,
}

impl OverlayKind {
    pub fn to_focus_context(&self) -> FocusContext {
        match self {
            OverlayKind::Details => FocusContext::Details,
            OverlayKind::Dialog => FocusContext::Dialog,
            OverlayKind::Confirm => FocusContext::Confirm,
            OverlayKind::Help => FocusContext::Help,
            OverlayKind::Notifications => FocusContext::Notifications,
            OverlayKind::ProposeTime => FocusContext::ProposeTime,
            OverlayKind::Goto => FocusContext::Goto,
            OverlayKind::MeetWith => FocusContext::MeetWith,
            OverlayKind::Accounts => FocusContext::Accounts,
            OverlayKind::CalDavLogin => FocusContext::CalDavLogin,
            OverlayKind::Messages => FocusContext::Messages,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Overlay {
    pub kind: OverlayKind,
    pub prev_focus: Option<FocusContext>,
}
