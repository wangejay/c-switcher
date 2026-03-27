use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command;

const KEYCHAIN_SERVICE: &str = "Claude Code-credentials";
const OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const OAUTH_TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";

fn keychain_account() -> String {
    std::env::var("USER").unwrap_or_else(|_| "wangejay".to_string())
}

fn claude_json_path() -> PathBuf {
    dirs::home_dir().unwrap().join(".claude.json")
}

pub fn profiles_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(".claude_profiles")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// ── Keychain helpers ──

async fn run_security(args: &[&str]) -> Result<String, String> {
    let output = Command::new("security")
        .args(args)
        .output()
        .await
        .map_err(|e| format!("Failed to run security: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("security command failed: {}", stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub async fn get_keychain_token() -> Result<String, String> {
    let account = keychain_account();
    run_security(&[
        "find-generic-password",
        "-s",
        KEYCHAIN_SERVICE,
        "-a",
        &account,
        "-w",
    ])
    .await
}

pub async fn set_keychain_token(token: &str) -> Result<(), String> {
    let account = keychain_account();
    // Delete existing (ignore error if not found)
    let _ = run_security(&[
        "delete-generic-password",
        "-s",
        KEYCHAIN_SERVICE,
        "-a",
        &account,
    ])
    .await;

    run_security(&[
        "add-generic-password",
        "-s",
        KEYCHAIN_SERVICE,
        "-a",
        &account,
        "-w",
        token,
    ])
    .await?;
    Ok(())
}

// ── OAuth account helpers ──

/// Represents the oauthAccount object in ~/.claude.json.
/// Uses `flatten` + HashMap to round-trip unknown fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthAccount {
    #[serde(default)]
    pub email_address: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub organization_name: String,
    #[serde(default)]
    pub organization_role: String,
    #[serde(default)]
    pub account_uuid: String,
    #[serde(default)]
    pub billing_type: String,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

pub fn read_oauth_account() -> Result<OAuthAccount, String> {
    let path = claude_json_path();
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    let root: serde_json::Value =
        serde_json::from_str(&data).map_err(|e| format!("Invalid JSON: {}", e))?;
    let oauth_val = root
        .get("oauthAccount")
        .cloned()
        .unwrap_or(serde_json::Value::Object(Default::default()));
    serde_json::from_value(oauth_val).map_err(|e| format!("Parse oauthAccount: {}", e))
}

pub fn write_oauth_account(oauth: &OAuthAccount) -> Result<(), String> {
    let path = claude_json_path();
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    let mut root: serde_json::Value =
        serde_json::from_str(&data).map_err(|e| format!("Invalid JSON: {}", e))?;
    let oauth_val = serde_json::to_value(oauth).map_err(|e| e.to_string())?;
    root["oauthAccount"] = oauth_val;
    let out = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
    std::fs::write(&path, format!("{}\n", out))
        .map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
    Ok(())
}

// ── Response types ──

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CurrentInfo {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    org_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    billing: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileEntry {
    pub name: String,
    pub email: String,
    pub organization: String,
    pub display_name: String,
    pub billing: String,
    pub expires_at: u64,
    pub is_expired: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token_preview: Option<String>,
}

impl OpResult {
    pub fn ok(message: &str) -> Self {
        OpResult {
            success: true,
            message: Some(message.to_string()),
            error: None,
            email: None,
            organization: None,
            from: None,
            to: None,
            expires_at: None,
            access_token_preview: None,
        }
    }
    pub fn err(error: &str) -> Self {
        OpResult {
            success: false,
            message: None,
            error: Some(error.to_string()),
            email: None,
            organization: None,
            from: None,
            to: None,
            expires_at: None,
            access_token_preview: None,
        }
    }
}

// ── Profile file structure ──

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileFile {
    pub name: String,
    pub keychain_token: String,
    pub oauth_account: OAuthAccount,
}

// ── Tauri Commands ──

#[tauri::command]
async fn get_current_info() -> CurrentInfo {
    match read_oauth_account() {
        Ok(oauth) => CurrentInfo {
            success: true,
            email: Some(if oauth.email_address.is_empty() {
                "N/A".into()
            } else {
                oauth.email_address
            }),
            display_name: Some(if oauth.display_name.is_empty() {
                "N/A".into()
            } else {
                oauth.display_name
            }),
            organization: Some(if oauth.organization_name.is_empty() {
                "N/A".into()
            } else {
                oauth.organization_name
            }),
            org_role: Some(if oauth.organization_role.is_empty() {
                "N/A".into()
            } else {
                oauth.organization_role
            }),
            account_uuid: Some(if oauth.account_uuid.is_empty() {
                "N/A".into()
            } else {
                oauth.account_uuid
            }),
            billing: Some(if oauth.billing_type.is_empty() {
                "N/A".into()
            } else {
                oauth.billing_type
            }),
            error: None,
        },
        Err(e) => CurrentInfo {
            success: false,
            email: None,
            display_name: None,
            organization: None,
            org_role: None,
            account_uuid: None,
            billing: None,
            error: Some(e),
        },
    }
}

pub async fn list_profiles_impl() -> Vec<ProfileEntry> {
    let dir = profiles_dir();
    if !dir.exists() {
        return vec![];
    }
    let mut entries: Vec<_> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "json")
                    .unwrap_or(false)
            })
            .collect(),
        Err(_) => return vec![],
    };
    entries.sort_by_key(|e| e.file_name());

    let mut result = vec![];
    for entry in entries {
        let path = entry.path();
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        match std::fs::read_to_string(&path).and_then(|s| {
            serde_json::from_str::<ProfileFile>(&s)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }) {
            Ok(pf) => {
                // keychain_token is a double-serialized JSON string
                let token_data: serde_json::Value =
                    serde_json::from_str(&pf.keychain_token).unwrap_or_default();
                let expires_at = token_data
                    .get("claudeAiOauth")
                    .and_then(|o| o.get("expiresAt"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let is_expired = if expires_at > 0 {
                    expires_at < now_ms()
                } else {
                    true
                };
                result.push(ProfileEntry {
                    name,
                    email: if pf.oauth_account.email_address.is_empty() {
                        "unknown".into()
                    } else {
                        pf.oauth_account.email_address
                    },
                    organization: pf.oauth_account.organization_name,
                    display_name: pf.oauth_account.display_name,
                    billing: pf.oauth_account.billing_type,
                    expires_at,
                    is_expired,
                });
            }
            Err(_) => {
                result.push(ProfileEntry {
                    name,
                    email: "error reading profile".into(),
                    organization: String::new(),
                    display_name: String::new(),
                    billing: String::new(),
                    expires_at: 0,
                    is_expired: true,
                });
            }
        }
    }
    result
}

#[tauri::command]
async fn list_profiles() -> Vec<ProfileEntry> {
    list_profiles_impl().await
}

#[tauri::command]
async fn backup_profile(name: String) -> OpResult {
    let dir = profiles_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return OpResult::err(&format!("Cannot create profiles dir: {}", e));
    }
    let token = match get_keychain_token().await {
        Ok(t) => t,
        Err(e) => return OpResult::err(&e),
    };
    let oauth = match read_oauth_account() {
        Ok(o) => o,
        Err(e) => return OpResult::err(&e),
    };

    let pf = ProfileFile {
        name: name.clone(),
        keychain_token: token,
        oauth_account: oauth,
    };
    let path = dir.join(format!("{}.json", name));
    let json = match serde_json::to_string_pretty(&pf) {
        Ok(j) => j,
        Err(e) => return OpResult::err(&e.to_string()),
    };
    if let Err(e) = std::fs::write(&path, format!("{}\n", json)) {
        return OpResult::err(&format!("Cannot write profile: {}", e));
    }

    let mut r = OpResult::ok(&format!("已備份 profile: {}", name));
    r.email = Some(pf.oauth_account.email_address);
    r.organization = Some(pf.oauth_account.organization_name);
    r
}

pub async fn switch_profile_impl(name: String) -> OpResult {
    let path = profiles_dir().join(format!("{}.json", name));
    if !path.exists() {
        return OpResult::err(&format!("Profile '{}' 不存在", name));
    }
    let pf: ProfileFile = match std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
    {
        Ok(p) => p,
        Err(e) => return OpResult::err(&e),
    };

    let current_oauth = read_oauth_account().unwrap_or(OAuthAccount {
        email_address: "unknown".into(),
        display_name: String::new(),
        organization_name: String::new(),
        organization_role: String::new(),
        account_uuid: String::new(),
        billing_type: String::new(),
        extra: HashMap::new(),
    });
    let from_email = if current_oauth.email_address.is_empty() {
        "unknown".to_string()
    } else {
        current_oauth.email_address.clone()
    };
    let to_email = if pf.oauth_account.email_address.is_empty() {
        "unknown".to_string()
    } else {
        pf.oauth_account.email_address.clone()
    };

    // Before switching away, save the current Keychain token back to the
    // outgoing profile file so it stays up-to-date (Claude Code may have
    // rotated the token since the profile was last saved).
    if let Ok(current_keychain) = get_keychain_token().await {
        let dir = profiles_dir();
        if dir.exists() {
            if let Ok(rd) = std::fs::read_dir(&dir) {
                for entry in rd.filter_map(|e| e.ok()) {
                    let ep = entry.path();
                    if ep.extension().map(|x| x == "json").unwrap_or(false) {
                        if let Ok(content) = std::fs::read_to_string(&ep) {
                            if let Ok(mut epf) =
                                serde_json::from_str::<ProfileFile>(&content)
                            {
                                if epf.oauth_account.email_address
                                    == current_oauth.email_address
                                    && epf.oauth_account.account_uuid
                                        == current_oauth.account_uuid
                                {
                                    epf.keychain_token = current_keychain.clone();
                                    if let Ok(json) = serde_json::to_string_pretty(&epf)
                                    {
                                        let _ =
                                            std::fs::write(&ep, format!("{}\n", json));
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Err(e) = set_keychain_token(&pf.keychain_token).await {
        return OpResult::err(&e);
    }
    if let Err(e) = write_oauth_account(&pf.oauth_account) {
        return OpResult::err(&e);
    }

    // Check if the switched-to token is expired and auto-refresh
    let token_data: serde_json::Value =
        serde_json::from_str(&pf.keychain_token).unwrap_or_default();
    let expires_at = token_data
        .get("claudeAiOauth")
        .and_then(|o| o.get("expiresAt"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let is_expired = expires_at == 0 || expires_at < now_ms();

    if is_expired {
        // Auto-refresh the profile token after switch
        let _ = refresh_token_impl(Some(name)).await;
    }

    let mut r = OpResult::ok("已切換帳號");
    r.from = Some(from_email);
    r.to = Some(to_email);
    r
}

#[tauri::command]
async fn switch_profile(name: String) -> OpResult {
    switch_profile_impl(name).await
}

#[tauri::command]
async fn delete_profile(name: String) -> OpResult {
    let path = profiles_dir().join(format!("{}.json", name));
    if !path.exists() {
        return OpResult::err(&format!("Profile '{}' 不存在", name));
    }
    match std::fs::remove_file(&path) {
        Ok(_) => OpResult::ok(&format!("已刪除 profile: {}", name)),
        Err(e) => OpResult::err(&e.to_string()),
    }
}

pub async fn refresh_token_impl(profile_name: Option<String>) -> OpResult {
    // Get the token JSON string
    let (token_str, profile_path) = if let Some(ref pname) = profile_name {
        let path = profiles_dir().join(format!("{}.json", pname));
        if !path.exists() {
            return OpResult::err(&format!("Profile '{}' 不存在", pname));
        }
        let pf: ProfileFile = match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        {
            Ok(p) => p,
            Err(e) => return OpResult::err(&e),
        };

        // If this profile is the currently active account, prefer the Keychain token
        // because Claude Code may have refreshed the token (rotating the refresh token),
        // making the profile file's refresh token stale/invalid.
        let token = if let Ok(current_oauth) = read_oauth_account() {
            if current_oauth.email_address == pf.oauth_account.email_address
                && current_oauth.account_uuid == pf.oauth_account.account_uuid
            {
                // Active account — use the latest token from Keychain
                get_keychain_token().await.unwrap_or(pf.keychain_token)
            } else {
                pf.keychain_token
            }
        } else {
            pf.keychain_token
        };
        (token, Some(path))
    } else {
        match get_keychain_token().await {
            Ok(t) => (t, None),
            Err(e) => return OpResult::err(&e),
        }
    };

    // Parse the double-serialized token
    let mut token_json: serde_json::Value = match serde_json::from_str(&token_str) {
        Ok(v) => v,
        Err(e) => return OpResult::err(&format!("Parse token failed: {}", e)),
    };

    let refresh_tok = match token_json
        .get("claudeAiOauth")
        .and_then(|o| o.get("refreshToken"))
        .and_then(|v| v.as_str())
    {
        Some(t) => t.to_string(),
        None => return OpResult::err("找不到 refreshToken"),
    };

    // POST to token endpoint
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": refresh_tok,
        "client_id": OAUTH_CLIENT_ID,
    });

    let resp = match client
        .post(OAUTH_TOKEN_URL)
        .header("Content-Type", "application/json")
        .header("User-Agent", "claude-code/2.1.0")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return OpResult::err(&format!("HTTP error: {}", e)),
    };

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body_text = resp.text().await.unwrap_or_default();
        return OpResult::err(&format!("HTTP {}: {}", status, body_text));
    }

    let resp_data: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return OpResult::err(&format!("Parse response: {}", e)),
    };

    let new_access = resp_data
        .get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let new_refresh = resp_data
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .unwrap_or(&refresh_tok)
        .to_string();
    let expires_in = resp_data
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(3600);
    let expires_at = now_ms() + expires_in * 1000;

    // Update token JSON
    if let Some(oauth_data) = token_json.get_mut("claudeAiOauth") {
        oauth_data["accessToken"] = serde_json::Value::String(new_access.clone());
        oauth_data["refreshToken"] = serde_json::Value::String(new_refresh);
        oauth_data["expiresAt"] = serde_json::json!(expires_at);
    }

    let new_token_str = serde_json::to_string(&token_json).unwrap_or_default();

    // Save back
    if let Some(ref path) = profile_path {
        // Update profile file
        let mut pf: ProfileFile = match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        {
            Ok(p) => p,
            Err(e) => return OpResult::err(&e),
        };
        pf.keychain_token = new_token_str.clone();
        let json = serde_json::to_string_pretty(&pf).unwrap_or_default();
        if let Err(e) = std::fs::write(path, format!("{}\n", json)) {
            return OpResult::err(&format!("Write profile: {}", e));
        }
        // If this profile is the currently active account, also update keychain
        // (refresh token rotation invalidates the old token)
        if let Ok(current_oauth) = read_oauth_account() {
            if current_oauth.email_address == pf.oauth_account.email_address
                && current_oauth.account_uuid == pf.oauth_account.account_uuid
            {
                if let Err(e) = set_keychain_token(&new_token_str).await {
                    return OpResult::err(&format!("Keychain sync failed: {}", e));
                }
            }
        }
    } else {
        if let Err(e) = set_keychain_token(&new_token_str).await {
            return OpResult::err(&e);
        }
    }

    // Format expiry
    let expires_secs = (expires_at / 1000) as i64;
    let naive =
        chrono_minimal_format(expires_secs);

    let mut r = OpResult::ok("Token 刷新完成");
    r.expires_at = Some(naive);
    if new_access.len() > 30 {
        r.access_token_preview = Some(format!(
            "{}...{}",
            &new_access[..20],
            &new_access[new_access.len() - 10..]
        ));
    }
    r
}

#[tauri::command]
async fn refresh_token(profile_name: Option<String>) -> OpResult {
    refresh_token_impl(profile_name).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Helper: extract access token from a keychain token JSON string
pub fn extract_access_token(token_str: &str) -> Result<String, String> {
    let token_json: serde_json::Value =
        serde_json::from_str(token_str).map_err(|e| format!("Parse token failed: {}", e))?;
    token_json
        .get("claudeAiOauth")
        .and_then(|o| o.get("accessToken"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "找不到 accessToken".to_string())
}

pub async fn get_usage_impl(profile_name: Option<String>) -> UsageResult {
    // Get the token string
    let token_str = if let Some(ref pname) = profile_name {
        let path = profiles_dir().join(format!("{}.json", pname));
        if !path.exists() {
            return UsageResult {
                success: false,
                error: Some(format!("Profile '{}' 不存在", pname)),
                data: None,
            };
        }
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| {
                serde_json::from_str::<ProfileFile>(&s).map_err(|e| e.to_string())
            }) {
            Ok(pf) => {
                // If this profile is the currently active account, use Keychain token
                // (Claude Code may have rotated the refresh/access token)
                if let Ok(current_oauth) = read_oauth_account() {
                    if current_oauth.email_address == pf.oauth_account.email_address
                        && current_oauth.account_uuid == pf.oauth_account.account_uuid
                    {
                        get_keychain_token().await.unwrap_or(pf.keychain_token)
                    } else {
                        pf.keychain_token
                    }
                } else {
                    pf.keychain_token
                }
            }
            Err(e) => {
                return UsageResult {
                    success: false,
                    error: Some(e),
                    data: None,
                }
            }
        }
    } else {
        match get_keychain_token().await {
            Ok(t) => t,
            Err(e) => {
                return UsageResult {
                    success: false,
                    error: Some(e),
                    data: None,
                }
            }
        }
    };

    // Extract access token
    let access_token = match extract_access_token(&token_str) {
        Ok(t) => t,
        Err(e) => {
            return UsageResult {
                success: false,
                error: Some(e),
                data: None,
            }
        }
    };

    // Call usage API
    let client = reqwest::Client::new();
    let resp = match client
        .get("https://api.anthropic.com/api/oauth/usage")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("anthropic-beta", "oauth-2025-04-20")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return UsageResult {
                success: false,
                error: Some(format!("HTTP error: {}", e)),
                data: None,
            }
        }
    };

    if resp.status().as_u16() == 401 {
        // Token expired — auto-refresh and retry once
        let refresh_result = refresh_token_impl(profile_name.clone()).await;
        if !refresh_result.success {
            return UsageResult {
                success: false,
                error: Some(format!(
                    "Token expired and refresh failed: {}",
                    refresh_result.error.unwrap_or_default()
                )),
                data: None,
            };
        }

        // Re-read the (now refreshed) token
        let new_token_str = if let Some(ref pname) = profile_name {
            let path = profiles_dir().join(format!("{}.json", pname));
            match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|s| {
                    serde_json::from_str::<ProfileFile>(&s).map_err(|e| e.to_string())
                }) {
                Ok(pf) => pf.keychain_token,
                Err(e) => {
                    return UsageResult {
                        success: false,
                        error: Some(e),
                        data: None,
                    }
                }
            }
        } else {
            match get_keychain_token().await {
                Ok(t) => t,
                Err(e) => {
                    return UsageResult {
                        success: false,
                        error: Some(e),
                        data: None,
                    }
                }
            }
        };

        let new_access = match extract_access_token(&new_token_str) {
            Ok(t) => t,
            Err(e) => {
                return UsageResult {
                    success: false,
                    error: Some(e),
                    data: None,
                }
            }
        };

        // Retry the usage API call with refreshed token
        let retry_resp = match client
            .get("https://api.anthropic.com/api/oauth/usage")
            .header("Authorization", format!("Bearer {}", new_access))
            .header("anthropic-beta", "oauth-2025-04-20")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return UsageResult {
                    success: false,
                    error: Some(format!("HTTP error on retry: {}", e)),
                    data: None,
                }
            }
        };

        if !retry_resp.status().is_success() {
            let status = retry_resp.status().as_u16();
            let body_text = retry_resp.text().await.unwrap_or_default();
            return UsageResult {
                success: false,
                error: Some(format!("HTTP {} (after refresh): {}", status, body_text)),
                data: None,
            };
        }

        return match retry_resp.json::<serde_json::Value>().await {
            Ok(data) => UsageResult {
                success: true,
                error: None,
                data: Some(data),
            },
            Err(e) => UsageResult {
                success: false,
                error: Some(format!("Parse response: {}", e)),
                data: None,
            },
        };
    }

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body_text = resp.text().await.unwrap_or_default();
        return UsageResult {
            success: false,
            error: Some(format!("HTTP {}: {}", status, body_text)),
            data: None,
        };
    }

    match resp.json::<serde_json::Value>().await {
        Ok(data) => UsageResult {
            success: true,
            error: None,
            data: Some(data),
        },
        Err(e) => UsageResult {
            success: false,
            error: Some(format!("Parse response: {}", e)),
            data: None,
        },
    }
}

#[tauri::command]
async fn get_usage(profile_name: Option<String>) -> UsageResult {
    get_usage_impl(profile_name).await
}

/// Minimal timestamp formatting without pulling in chrono crate
fn chrono_minimal_format(epoch_secs: i64) -> String {
    // Use the `date` command on macOS for formatting
    // But since we're in an async context, just return the epoch-based string
    // We'll compute it manually
    let secs = epoch_secs;
    // Simple approach: return ISO-ish format
    // Since we don't have chrono, we'll just format the timestamp
    // using basic arithmetic for UTC
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calculate year/month/day from days since epoch (1970-01-01)
    let mut remaining_days = days_since_epoch;
    let mut year = 1970i64;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }
    let days_in_months: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 0usize;
    for (i, &d) in days_in_months.iter().enumerate() {
        if remaining_days < d {
            month = i;
            break;
        }
        remaining_days -= d;
    }
    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year,
        month + 1,
        day,
        hours,
        minutes,
        seconds
    )
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_current_info,
            list_profiles,
            backup_profile,
            switch_profile,
            delete_profile,
            refresh_token,
            get_usage,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
