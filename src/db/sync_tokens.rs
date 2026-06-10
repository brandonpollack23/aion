use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const SYNC_KEY_SEP: char = '\t';

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncTokenStore {
    pub tokens: HashMap<String, String>,
    #[serde(rename = "lastFullSync")]
    pub last_full_sync: Option<String>,
}

pub fn sync_tokens_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()))
        .join("aion")
        .join("sync_tokens.json")
}

pub fn load_sync_tokens() -> SyncTokenStore {
    let path = sync_tokens_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => SyncTokenStore::default(),
    }
}

pub fn save_sync_tokens(store: &SyncTokenStore) -> Result<()> {
    let path = sync_tokens_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(store)?;
    std::fs::write(&path, json)?;
    Ok(())
}

pub fn sync_key(account_email: &str, calendar_id: &str) -> String {
    format!("{}{}{}", account_email, SYNC_KEY_SEP, calendar_id)
}

pub fn get_sync_token(account_email: &str, calendar_id: &str) -> Option<String> {
    let store = load_sync_tokens();
    store
        .tokens
        .get(&sync_key(account_email, calendar_id))
        .cloned()
}

pub fn set_sync_token(account_email: &str, calendar_id: &str, token: &str) -> Result<()> {
    let mut store = load_sync_tokens();
    store
        .tokens
        .insert(sync_key(account_email, calendar_id), token.to_string());
    save_sync_tokens(&store)
}

pub fn clear_all_sync_tokens() -> Result<()> {
    save_sync_tokens(&SyncTokenStore::default())
}
