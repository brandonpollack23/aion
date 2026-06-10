// Field indices
pub const CD_NAME: usize = 0;
pub const CD_EMAIL: usize = 1;
pub const CD_SERVER_URL: usize = 2;
pub const CD_USERNAME: usize = 3;
pub const CD_PASSWORD: usize = 4;
pub const CD_PASSWORD_CMD: usize = 5;
pub const CD_FIELD_COUNT: usize = 6;

pub const PRESET_NAMES: &[&str] = &[
    "iCloud",
    "Fastmail",
    "Nextcloud",
    "Radicale",
    "Baïkal",
    "Custom",
];

pub const PRESET_URLS: &[&str] = &[
    "https://caldav.icloud.com",
    "https://caldav.fastmail.com",
    "",
    "",
    "",
    "",
];

#[derive(Debug, Clone)]
pub struct CalDavDialogState {
    pub fields: [String; CD_FIELD_COUNT],
    pub active_field: usize,
    pub preset_idx: usize,
    pub testing: bool,
    pub test_result: Option<Result<String, String>>,
    pub saving: bool,
}

impl CalDavDialogState {
    pub fn new() -> Self {
        Self {
            fields: [
                String::new(), // name
                String::new(), // email
                String::new(), // server_url
                String::new(), // username
                String::new(), // password
                String::new(), // password_command
            ],
            active_field: CD_NAME,
            preset_idx: 5, // Custom
            testing: false,
            test_result: None,
            saving: false,
        }
    }

    pub fn apply_preset(&mut self, idx: usize) {
        self.preset_idx = idx;
        if let Some(url) = PRESET_URLS.get(idx) {
            if !url.is_empty() {
                self.fields[CD_SERVER_URL] = url.to_string();
            }
        }
    }

    pub fn active_field_value(&self) -> &str {
        &self.fields[self.active_field]
    }

    pub fn active_field_value_mut(&mut self) -> &mut String {
        &mut self.fields[self.active_field]
    }

    pub fn insert_char(&mut self, ch: char) {
        self.fields[self.active_field].push(ch);
    }

    pub fn delete_char(&mut self) {
        self.fields[self.active_field].pop();
    }

    pub fn next_field(&mut self) {
        self.active_field = (self.active_field + 1) % CD_FIELD_COUNT;
    }

    pub fn prev_field(&mut self) {
        self.active_field = (self.active_field + CD_FIELD_COUNT - 1) % CD_FIELD_COUNT;
    }

    pub fn is_valid(&self) -> bool {
        !self.fields[CD_EMAIL].trim().is_empty()
            && !self.fields[CD_SERVER_URL].trim().is_empty()
            && !self.fields[CD_USERNAME].trim().is_empty()
            && (!self.fields[CD_PASSWORD].trim().is_empty()
                || !self.fields[CD_PASSWORD_CMD].trim().is_empty())
    }

    pub fn to_config_entry(&self) -> crate::config::schema::CalDAVAccount {
        let name = if self.fields[CD_NAME].trim().is_empty() {
            self.fields[CD_EMAIL].trim().to_string()
        } else {
            self.fields[CD_NAME].trim().to_string()
        };
        crate::config::schema::CalDAVAccount {
            name,
            email: self.fields[CD_EMAIL].trim().to_string(),
            server_url: self.fields[CD_SERVER_URL].trim().to_string(),
            username: self.fields[CD_USERNAME].trim().to_string(),
            password: {
                let p = self.fields[CD_PASSWORD].trim().to_string();
                if p.is_empty() {
                    None
                } else {
                    Some(p)
                }
            },
            password_command: {
                let p = self.fields[CD_PASSWORD_CMD].trim().to_string();
                if p.is_empty() {
                    None
                } else {
                    Some(p)
                }
            },
        }
    }
}
