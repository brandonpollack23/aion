use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    Info,
    Success,
    Warning,
    Error,
    Progress,
}

#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub current: Option<u32>,
    pub total: Option<u32>,
    pub phase: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub text: String,
    pub msg_type: MessageType,
    pub progress: Option<ProgressInfo>,
    pub created_at: Instant,
}

impl Message {
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            id: uuid(),
            text: text.into(),
            msg_type: MessageType::Info,
            progress: None,
            created_at: Instant::now(),
        }
    }

    pub fn success(text: impl Into<String>) -> Self {
        Self {
            id: uuid(),
            text: text.into(),
            msg_type: MessageType::Success,
            progress: None,
            created_at: Instant::now(),
        }
    }

    pub fn warning(text: impl Into<String>) -> Self {
        Self {
            id: uuid(),
            text: text.into(),
            msg_type: MessageType::Warning,
            progress: None,
            created_at: Instant::now(),
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            id: uuid(),
            text: text.into(),
            msg_type: MessageType::Error,
            progress: None,
            created_at: Instant::now(),
        }
    }
}

fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{:x}", nanos)
}
