use std::path::PathBuf;
use std::sync::OnceLock;

const LOGO_BYTES: &[u8] = include_bytes!("../images/logo.png");

static ICON_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

fn icon_path() -> Option<&'static PathBuf> {
    ICON_PATH
        .get_or_init(|| {
            let path = std::env::temp_dir().join("aion_logo.png");
            std::fs::write(&path, LOGO_BYTES).ok()?;
            Some(path)
        })
        .as_ref()
}

pub fn send(title: &str, body: &str) {
    let mut n = notify_rust::Notification::new();
    n.summary(title).body(body).appname("aion");
    if let Some(p) = icon_path() {
        n.icon(&p.to_string_lossy());
    } else {
        n.icon("calendar");
    }
    if let Err(e) = n.show() {
        tracing::warn!("Desktop notification failed: {}", e);
    }
}
