use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::auth::tokens::{now_ms, save_account_tokens, AccountInfo, AccountType, TokenData};

const REDIRECT_URI: &str = "http://localhost:8085/callback";
const AUTH_PORT: u16 = 8085;
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/calendar.events",
    "https://www.googleapis.com/auth/calendar.readonly",
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
];

pub struct LoginResult {
    pub account: AccountInfo,
}

fn generate_code_verifier() -> String {
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

fn generate_code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

fn generate_state() -> String {
    let mut buf = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

fn build_auth_url(client_id: &str, state: &str, code_challenge: &str) -> String {
    use url::Url;
    let mut u = Url::parse(AUTH_URL).unwrap();
    u.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("response_type", "code")
        .append_pair("scope", &SCOPES.join(" "))
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .append_pair("state", state)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256");
    u.to_string()
}

fn success_html(email: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><title>Aion - Login Successful</title></head>
<body style="font-family:sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;background:#1a1a2e;color:#fff">
<div style="text-align:center">
<div style="font-size:4rem">&#x2705;</div>
<h1>Login Successful!</h1>
<p>You can close this window and return to Aion.</p>
<div style="margin-top:1rem;padding:0.5rem 1rem;background:rgba(255,255,255,0.1);border-radius:8px;font-family:monospace">{}</div>
</div></body></html>"#,
        email
    )
}

fn error_html(error: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><title>Aion - Login Failed</title></head>
<body style="font-family:sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;background:#2e1a1a;color:#fff">
<div style="text-align:center">
<div style="font-size:4rem">&#x274C;</div>
<h1>Login Failed</h1>
<div style="margin-top:1rem;padding:1rem;background:rgba(255,255,255,0.1);border-radius:8px;font-family:monospace">{}</div>
</div></body></html>"#,
        error
    )
}

fn send_html_response(status: u16, reason: &str, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status, reason, body.len(), body
    ).into_bytes()
}

fn parse_callback_params(request: &str) -> HashMap<String, String> {
    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("");
    let query = path.splitn(2, '?').nth(1).unwrap_or("");

    // Use url crate for proper percent-decoding
    let dummy = format!("http://localhost{}", path);
    if let Ok(u) = url::Url::parse(&dummy) {
        return u.query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
    }

    // Fallback: manual split
    query
        .split('&')
        .filter_map(|kv| {
            let mut parts = kv.splitn(2, '=');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect()
}

pub async fn start_login_flow(
    client_id: String,
    client_secret: String,
    on_auth_url: impl Fn(String) + Send + 'static,
) -> Result<LoginResult> {
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);
    let state = generate_state();
    let auth_url = build_auth_url(&client_id, &state, &code_challenge);

    on_auth_url(auth_url.clone());
    let _ = open::that(&auth_url);

    let listener = TcpListener::bind(("127.0.0.1", AUTH_PORT))
        .await
        .context("Failed to bind OAuth callback port 8085. Is another process using it?")?;

    tokio::time::timeout(Duration::from_secs(300), async move {
        loop {
            let (mut stream, _) = listener.accept().await?;

            let mut buf = [0u8; 8192];
            let n = stream.read(&mut buf).await.unwrap_or(0);
            let request = std::str::from_utf8(&buf[..n]).unwrap_or("");

            // Ignore requests that are not to /callback
            let first_line = request.lines().next().unwrap_or("");
            let path = first_line.split_whitespace().nth(1).unwrap_or("");
            if !path.starts_with("/callback") {
                let resp = send_html_response(404, "Not Found", "Not found");
                let _ = stream.write_all(&resp).await;
                continue;
            }

            let params = parse_callback_params(request);

            if let Some(error) = params.get("error") {
                let html = error_html(error);
                let _ = stream.write_all(&send_html_response(200, "OK", &html)).await;
                anyhow::bail!("Google returned error: {}", error);
            }

            if params.get("state").map(|s| s.as_str()) != Some(state.as_str()) {
                let html = error_html("Invalid state parameter (CSRF check failed)");
                let _ = stream.write_all(&send_html_response(200, "OK", &html)).await;
                anyhow::bail!("Invalid state parameter");
            }

            let code = params.get("code").cloned().unwrap_or_default();
            if code.is_empty() {
                let html = error_html("No authorization code received");
                let _ = stream.write_all(&send_html_response(200, "OK", &html)).await;
                anyhow::bail!("No authorization code received");
            }

            match exchange_code(&client_id, &client_secret, &code, &code_verifier).await {
                Ok(tokens) => match fetch_user_info(&tokens.access_token).await {
                    Ok(user_info) => {
                        save_account_tokens(&user_info, tokens)?;
                        let html = success_html(&user_info.email);
                        let _ = stream.write_all(&send_html_response(200, "OK", &html)).await;
                        return Ok(LoginResult { account: user_info });
                    }
                    Err(e) => {
                        let html = error_html(&e.to_string());
                        let _ = stream.write_all(&send_html_response(200, "OK", &html)).await;
                        return Err(e);
                    }
                },
                Err(e) => {
                    let html = error_html(&e.to_string());
                    let _ = stream.write_all(&send_html_response(200, "OK", &html)).await;
                    return Err(e);
                }
            }
        }
    })
    .await
    .context("Login timed out after 5 minutes")?
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: String,
    expires_in: u64,
    scope: Option<String>,
}

async fn exchange_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
    code_verifier: &str,
) -> Result<TokenData> {
    let client = reqwest::Client::new();
    let res = client
        .post(TOKEN_URL)
        .form(&[
            ("code", code),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", REDIRECT_URI),
            ("grant_type", "authorization_code"),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .context("Token exchange request failed")?;

    if !res.status().is_success() {
        let text = res.text().await.unwrap_or_default();
        anyhow::bail!("Token exchange failed: {}", text);
    }

    let resp: TokenResponse = res.json().await.context("Failed to parse token response")?;
    let refresh_token = resp.refresh_token.ok_or_else(|| {
        anyhow::anyhow!(
            "Google did not provide a refresh token. \
             Please revoke Aion's access at https://myaccount.google.com/permissions and try again."
        )
    })?;

    Ok(TokenData {
        access_token: resp.access_token,
        refresh_token,
        token_type: resp.token_type,
        expires_in: resp.expires_in,
        scope: resp.scope.unwrap_or_default(),
        expires_at: Some(now_ms() + resp.expires_in * 1000),
    })
}

#[derive(Deserialize)]
struct UserInfoResponse {
    email: String,
    name: Option<String>,
    picture: Option<String>,
}

async fn fetch_user_info(access_token: &str) -> Result<AccountInfo> {
    let client = reqwest::Client::new();
    let res = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .context("User info request failed")?;

    if !res.status().is_success() {
        anyhow::bail!("Failed to fetch user info: {}", res.status());
    }

    let info: UserInfoResponse = res.json().await.context("Failed to parse user info")?;
    Ok(AccountInfo {
        email: info.email,
        name: info.name,
        picture: info.picture,
        account_type: AccountType::Google,
    })
}
