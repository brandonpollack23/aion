pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
}

pub static ALL_COMMANDS: &[Command] = &[
    Command {
        name: "new",
        description: "Create a new event",
    },
    Command {
        name: "new <title>",
        description: "Create event with title",
    },
    Command {
        name: "edit",
        description: "Edit selected event",
    },
    Command {
        name: "delete",
        description: "Delete selected event",
    },
    Command {
        name: "goto",
        description: "Go to today",
    },
    Command {
        name: "goto <date>",
        description: "Jump to date (e.g. 'next friday')",
    },
    Command {
        name: "sync",
        description: "Force sync all calendars",
    },
    Command {
        name: "login",
        description: "Add a Google account",
    },
    Command {
        name: "logout",
        description: "Remove all Google accounts",
    },
    Command {
        name: "accounts",
        description: "Manage connected accounts",
    },
    Command {
        name: "caldav",
        description: "Add a CalDAV account",
    },
    Command {
        name: "columns <n>",
        description: "Set number of columns (1, 3, or 5)",
    },
    Command {
        name: "3",
        description: "Toggle 3-column view",
    },
    Command {
        name: "notifications",
        description: "Show pending invitations",
    },
    Command {
        name: "messages",
        description: "Show message history",
    },
    Command {
        name: "meet",
        description: "Find time to meet with someone",
    },
    Command {
        name: "meet <email>",
        description: "Find time with specific person",
    },
    Command {
        name: "quit",
        description: "Exit aion",
    },
];

pub fn filter_commands(query: &str) -> Vec<&'static Command> {
    if query.is_empty() {
        return ALL_COMMANDS.iter().collect();
    }
    let q = query.to_lowercase();
    let first_word = q.split_whitespace().next().unwrap_or("");
    ALL_COMMANDS
        .iter()
        .filter(|cmd| {
            let cmd_name = cmd.name.split_whitespace().next().unwrap_or("");
            cmd_name.contains(first_word) || cmd.description.to_lowercase().contains(first_word)
        })
        .collect()
}
