use anyhow::{Context, Result};

/// Resolve a password from either a plain string or a shell command.
/// `password_command` takes precedence.
pub async fn resolve_password(
    password: Option<&str>,
    password_command: Option<&str>,
    label: Option<&str>,
) -> Result<String> {
    if let Some(cmd) = password_command {
        let output = tokio::process::Command::new("sh")
            .args(["-c", cmd])
            .output()
            .await
            .with_context(|| {
                format!(
                    "Failed to run password_command{}",
                    label.map(|l| format!(" for {}", l)).unwrap_or_default()
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "password_command exited with error{}: {}",
                label.map(|l| format!(" ({})", l)).unwrap_or_default(),
                stderr.trim()
            );
        }

        let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if resolved.is_empty() {
            anyhow::bail!(
                "password_command returned empty output{}",
                label.map(|l| format!(" for {}", l)).unwrap_or_default()
            );
        }
        return Ok(resolved);
    }

    if let Some(pw) = password {
        if !pw.is_empty() {
            return Ok(pw.to_string());
        }
    }

    anyhow::bail!(
        "No password configured{}. Set either 'password' or 'password_command' in config.toml",
        label.map(|l| format!(" for {}", l)).unwrap_or_default()
    )
}
