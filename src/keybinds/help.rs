pub struct KeybindDef {
    pub display: &'static str,
    pub description: &'static str,
}

pub struct Section {
    pub title: &'static str,
    pub binds: &'static [KeybindDef],
}

pub static GLOBAL_BINDS: Section = Section {
    title: "Global",
    binds: &[
        KeybindDef {
            display: ":",
            description: "Open command bar",
        },
        KeybindDef {
            display: "/",
            description: "Search events",
        },
        KeybindDef {
            display: "?",
            description: "Show this help",
        },
        KeybindDef {
            display: "N",
            description: "New event",
        },
        KeybindDef {
            display: "C",
            description: "Toggle calendars sidebar",
        },
        KeybindDef {
            display: "I",
            description: "Open notifications / pending invites",
        },
        KeybindDef {
            display: "M",
            description: "Toggle month / day view",
        },
        KeybindDef {
            display: "H / L",
            description: "Previous / next month",
        },
        KeybindDef {
            display: "Ctrl+M",
            description: "Open Meet With (schedule a meeting)",
        },
        KeybindDef {
            display: "3",
            description: "Cycle column view (1→3→5→1)",
        },
        KeybindDef {
            display: "Z",
            description: "Toggle timezone display in header",
        },
        KeybindDef {
            display: "n",
            description: "Jump to now (today, current time)",
        },
        KeybindDef {
            display: "a",
            description: "Toggle all-day events expanded",
        },
        KeybindDef {
            display: "`",
            description: "Cycle focus (timeline → sidebars)",
        },
        KeybindDef {
            display: "q / Ctrl+C",
            description: "Quit",
        },
    ],
};

pub static TIMELINE_BINDS: Section = Section {
    title: "Timeline",
    binds: &[
        KeybindDef {
            display: "j / ↓",
            description: "Next event",
        },
        KeybindDef {
            display: "k / ↑",
            description: "Previous event",
        },
        KeybindDef {
            display: "h / ←",
            description: "Previous day / column",
        },
        KeybindDef {
            display: "l / →",
            description: "Next day / column",
        },
        KeybindDef {
            display: "gg",
            description: "First event",
        },
        KeybindDef {
            display: "G",
            description: "Last event",
        },
        KeybindDef {
            display: "Enter",
            description: "Open event details",
        },
        KeybindDef {
            display: "e",
            description: "Edit selected event",
        },
        KeybindDef {
            display: "N",
            description: "New event",
        },
        KeybindDef {
            display: "D",
            description: "Delete event",
        },
    ],
};

pub static DIALOG_BINDS: Section = Section {
    title: "Event dialog",
    binds: &[
        KeybindDef {
            display: "Tab",
            description: "Next field",
        },
        KeybindDef {
            display: "Shift+Tab",
            description: "Previous field",
        },
        KeybindDef {
            display: "Enter",
            description: "Apply / confirm field",
        },
        KeybindDef {
            display: "Space",
            description: "Toggle checkbox / cycle type",
        },
        KeybindDef {
            display: "Ctrl+S",
            description: "Save event",
        },
        KeybindDef {
            display: "Esc",
            description: "Cancel / discard",
        },
    ],
};

pub static GOTO_BINDS: Section = Section {
    title: "Go to date",
    binds: &[
        KeybindDef {
            display: "Enter",
            description: "Navigate to parsed date",
        },
        KeybindDef {
            display: "Esc",
            description: "Cancel",
        },
    ],
};

pub static MESSAGES_BINDS: Section = Section {
    title: "Messages",
    binds: &[
        KeybindDef {
            display: "c",
            description: "Clear message history",
        },
        KeybindDef {
            display: "Esc",
            description: "Close",
        },
    ],
};

pub static DAYS_BINDS: Section = Section {
    title: "Days sidebar",
    binds: &[
        KeybindDef {
            display: "j / k",
            description: "Navigate days",
        },
        KeybindDef {
            display: "gg / G",
            description: "First / last visible day",
        },
        KeybindDef {
            display: "Enter",
            description: "Select day, focus timeline",
        },
    ],
};

pub static CALENDARS_BINDS: Section = Section {
    title: "Calendars sidebar",
    binds: &[
        KeybindDef {
            display: "j / k",
            description: "Navigate calendars",
        },
        KeybindDef {
            display: "Space",
            description: "Toggle calendar visibility",
        },
    ],
};

pub static DETAILS_BINDS: Section = Section {
    title: "Event details",
    binds: &[
        KeybindDef {
            display: "y",
            description: "Accept invitation (RSVP)",
        },
        KeybindDef {
            display: "n",
            description: "Decline invitation (RSVP)",
        },
        KeybindDef {
            display: "m",
            description: "Maybe / tentative (RSVP)",
        },
        KeybindDef {
            display: "p",
            description: "Propose new time",
        },
        KeybindDef {
            display: "e",
            description: "Edit event",
        },
        KeybindDef {
            display: "D",
            description: "Delete event",
        },
        KeybindDef {
            display: "o",
            description: "Open meeting link",
        },
        KeybindDef {
            display: "t",
            description: "Toggle timezone display",
        },
        KeybindDef {
            display: "Esc",
            description: "Close panel",
        },
    ],
};

pub static SEARCH_BINDS: Section = Section {
    title: "Search",
    binds: &[
        KeybindDef {
            display: "↑ / ↓",
            description: "Navigate results",
        },
        KeybindDef {
            display: "Enter",
            description: "Jump to event",
        },
        KeybindDef {
            display: "Esc",
            description: "Close search",
        },
    ],
};

pub static COMMAND_BINDS: Section = Section {
    title: "Command bar",
    binds: &[
        KeybindDef {
            display: "↑ / ↓",
            description: "Navigate history / completions",
        },
        KeybindDef {
            display: "Tab",
            description: "Fill selected completion",
        },
        KeybindDef {
            display: "Enter",
            description: "Execute command",
        },
        KeybindDef {
            display: "Esc",
            description: "Cancel",
        },
    ],
};

pub static MONTH_BINDS: Section = Section {
    title: "Month view",
    binds: &[
        KeybindDef {
            display: "h / l",
            description: "Previous / next day",
        },
        KeybindDef {
            display: "j / k",
            description: "Previous / next week",
        },
        KeybindDef {
            display: "H / L",
            description: "Previous / next month",
        },
        KeybindDef {
            display: "n",
            description: "Jump to today",
        },
        KeybindDef {
            display: "Enter",
            description: "Switch to day view on selected day",
        },
        KeybindDef {
            display: "M",
            description: "Switch back to day view",
        },
    ],
};
