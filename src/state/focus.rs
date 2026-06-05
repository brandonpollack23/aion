#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusContext {
    Timeline,
    Days,
    Calendars,
    Details,
    Dialog,
    Command,
    Confirm,
    Notifications,
    Search,
    Help,
    Goto,
    MeetWith,
    Accounts,
    CalDavLogin,
    Messages,
    ProposeTime,
    LoginPrompt,
}

impl Default for FocusContext {
    fn default() -> Self {
        FocusContext::Timeline
    }
}
