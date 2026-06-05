use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: String,
    pub expires_at: Option<u64>, // Unix ms
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccountType {
    #[serde(rename = "google")]
    Google,
    #[serde(rename = "caldav")]
    CalDAV,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Google
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
    #[serde(rename = "type", default)]
    pub account_type: AccountType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub account: AccountInfo,
    pub tokens: Option<TokenData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountsStore {
    pub accounts: HashMap<String, AccountData>,
    #[serde(rename = "defaultAccount")]
    pub default_account: Option<String>,
}

fn accounts_file() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()))
        .join("aion")
        .join("accounts.json")
}

pub fn load_accounts_store() -> AccountsStore {
    let path = accounts_file();
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AccountsStore::default(),
    }
}

pub fn save_accounts_store(store: &AccountsStore) -> Result<()> {
    let path = accounts_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(store)?;
    std::fs::write(&path, json).context("Failed to write accounts.json")?;
    Ok(())
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn is_token_expired(tokens: &TokenData) -> bool {
    match tokens.expires_at {
        Some(exp) => now_ms() > exp.saturating_sub(5 * 60 * 1000),
        None => true,
    }
}

pub fn get_accounts() -> Vec<AccountData> {
    let mut accounts: Vec<AccountData> = load_accounts_store()
        .accounts
        .into_values()
        .collect();

    // Include CalDAV accounts from config.toml
    let config = crate::config::loader::load_config();
    for cal in &config.caldav {
        accounts.push(AccountData {
            account: AccountInfo {
                email: cal.email.clone(),
                name: Some(cal.name.clone()),
                picture: None,
                account_type: AccountType::CalDAV,
            },
            tokens: None,
        });
    }
    accounts
}

pub fn save_caldav_account_to_store(
    email: &str,
    name: &str,
    server_url: &str,
    username: &str,
    password: Option<&str>,
    password_command: Option<&str>,
) -> anyhow::Result<()> {
    use crate::config::schema::CalDAVAccount;
    let mut config = crate::config::loader::load_config();

    // Remove existing entry with same email if any
    config.caldav.retain(|c| c.email != email);
    config.caldav.push(CalDAVAccount {
        name: name.to_string(),
        email: email.to_string(),
        server_url: server_url.to_string(),
        username: username.to_string(),
        password: password.map(|s| s.to_string()),
        password_command: password_command.map(|s| s.to_string()),
    });
    crate::config::loader::save_config(&config)
}

pub fn remove_caldav_account(email: &str) -> anyhow::Result<()> {
    let mut config = crate::config::loader::load_config();
    config.caldav.retain(|c| c.email != email);
    crate::config::loader::save_config(&config)
}

pub fn save_account_tokens(info: &AccountInfo, tokens: TokenData) -> Result<()> {
    let mut store = load_accounts_store();
    let tokens_with_expiry = TokenData {
        expires_at: Some(now_ms() + tokens.expires_in * 1000),
        ..tokens
    };
    store.accounts.insert(
        info.email.clone(),
        AccountData {
            account: info.clone(),
            tokens: Some(tokens_with_expiry),
        },
    );
    if store.default_account.is_none() {
        store.default_account = Some(info.email.clone());
    }
    save_accounts_store(&store)
}

pub fn remove_account(email: &str) -> Result<()> {
    // Try removing from CalDAV config first
    let config = crate::config::loader::load_config();
    let is_caldav = config.caldav.iter().any(|c| c.email == email);
    if is_caldav {
        return remove_caldav_account(email);
    }
    // Otherwise remove from accounts.json (Google)
    let mut store = load_accounts_store();
    store.accounts.remove(email);
    if store.default_account.as_deref() == Some(email) {
        store.default_account = store.accounts.keys().next().cloned();
    }
    save_accounts_store(&store)
}

pub async fn refresh_access_token(
    email: &str,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<TokenData> {
    let client = reqwest::Client::new();
    let res = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .context("Token refresh request failed")?;

    if !res.status().is_success() {
        let text = res.text().await.unwrap_or_default();
        if text.contains("invalid_grant") {
            anyhow::bail!("Token expired or revoked for {}. Run ':login' to re-authenticate.", email);
        }
        anyhow::bail!("Token refresh failed: {}", text);
    }

    #[derive(Deserialize)]
    struct Resp {
        access_token: String,
        expires_in: u64,
        token_type: String,
        scope: Option<String>,
    }
    let resp: Resp = res.json().await?;

    let new_tokens = TokenData {
        access_token: resp.access_token,
        refresh_token: refresh_token.to_string(),
        token_type: resp.token_type,
        expires_in: resp.expires_in,
        scope: resp.scope.unwrap_or_default(),
        expires_at: Some(now_ms() + resp.expires_in * 1000),
    };

    // Persist refreshed token
    let mut store = load_accounts_store();
    if let Some(entry) = store.accounts.get_mut(email) {
        entry.tokens = Some(new_tokens.clone());
    }
    let _ = save_accounts_store(&store);

    Ok(new_tokens)
}

pub async fn get_valid_access_token(
    email: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<Option<String>> {
    let store = load_accounts_store();
    let data = match store.accounts.get(email) {
        Some(d) => d.clone(),
        None => return Ok(None),
    };
    if data.account.account_type == AccountType::CalDAV {
        return Ok(None);
    }
    let tokens = match data.tokens {
        Some(t) => t,
        None => return Ok(None),
    };
    if !is_token_expired(&tokens) {
        return Ok(Some(tokens.access_token));
    }
    let refreshed = refresh_access_token(email, &tokens.refresh_token, client_id, client_secret).await?;
    Ok(Some(refreshed.access_token))
}
