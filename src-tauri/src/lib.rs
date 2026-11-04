use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
#[cfg(target_os = "macos")]
use tauri::{ActivationPolicy, RunEvent};
use tokio::process::Command;
use tokio::time::{sleep, Duration};

const KEYCHAIN_SERVICE: &str = "Claude Code-credentials";
const OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
/// Same endpoint Claude Code uses; the old console.anthropic.com host keeps
/// answering refreshes with HTTP 429.
/// User-Agent for calls to Anthropic's OAuth endpoints (see post_token_refresh).
const REFRESH_USER_AGENT: &str = "axios/1.9.0";
const OAUTH_TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
/// Skip a non-forced refresh while the token still has at least this long to live.
const REFRESH_SKIP_MARGIN_MS: u64 = 5 * 60 * 1000;
/// Attempts against the token endpoint before giving up on HTTP 429.
const REFRESH_MAX_ATTEMPTS: u32 = 3;
/// Longest pause accepted between attempts; a larger Retry-After is reported instead.
const REFRESH_MAX_WAIT_SECS: u64 = 10;

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
    /// Human-readable subscription plan, e.g. "Max 20x", "Pro"; empty if unknown.
    pub plan: String,
}

/// Derive a plan label from the stored OAuth token's subscriptionType / rateLimitTier.
pub fn plan_label(token_data: &serde_json::Value) -> String {
    let oauth = token_data.get("claudeAiOauth");
    let field = |k: &str| {
        oauth
            .and_then(|o| o.get(k))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase()
    };
    let sub = field("subscriptionType");
    let tier = field("rateLimitTier");

    if tier.contains("max_20x") {
        return "Max 20x".into();
    }
    if tier.contains("max_5x") {
        return "Max 5x".into();
    }
    match sub.as_str() {
        "" => String::new(),
        "max" => "Max".into(),
        "pro" => "Pro".into(),
        "team" => "Team".into(),
        "enterprise" => "Enterprise".into(),
        other => {
            let mut c = other.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        }
    }
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
    /// True when a refresh was not needed (token still valid) and no request was made.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<bool>,
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
            skipped: None,
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
            skipped: None,
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

/// `expiresAt` (ms) of a stored keychain token JSON; 0 if unreadable.
fn token_expires_at(token: &str) -> i64 {
    serde_json::from_str::<serde_json::Value>(token)
        .ok()
        .and_then(|v| v.get("claudeAiOauth")?.get("expiresAt")?.as_i64())
        .unwrap_or(0)
}

/// Copy the live keychain token back into the profile file of the account
/// Claude Code is signed in with. Claude Code rotates the refresh token every
/// time it refreshes, which revokes the copy saved at backup time; without
/// this, a profile switched away from is left holding a dead token.
/// Returns the names of the profiles that were updated.
pub async fn sync_active_profile() -> Result<Vec<String>, String> {
    // Nobody signed in (or no keychain entry yet): nothing to save.
    let Ok(current) = read_oauth_account() else {
        return Ok(vec![]);
    };
    if current.email_address.is_empty() {
        return Ok(vec![]);
    }
    let Ok(live) = get_keychain_token().await else {
        return Ok(vec![]);
    };
    let live_expires = token_expires_at(&live);

    let mut synced = vec![];
    let rd = match std::fs::read_dir(profiles_dir()) {
        Ok(rd) => rd,
        Err(_) => return Ok(synced),
    };
    for path in rd.filter_map(|e| e.ok()).map(|e| e.path()) {
        if path.extension().map(|e| e != "json").unwrap_or(true) {
            continue;
        }
        let Ok(mut pf) = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str::<ProfileFile>(&s).map_err(|e| e.to_string()))
        else {
            continue;
        };
        if pf.oauth_account.email_address != current.email_address
            || pf.oauth_account.account_uuid != current.account_uuid
            || pf.keychain_token == live
            // Never replace a newer token (e.g. one refreshed by this app).
            || token_expires_at(&pf.keychain_token) > live_expires
        {
            continue;
        }
        pf.keychain_token = live.clone();
        let json = serde_json::to_string_pretty(&pf).map_err(|e| e.to_string())?;
        std::fs::write(&path, format!("{}\n", json))
            .map_err(|e| format!("Write profile: {}", e))?;
        synced.push(pf.name);
    }
    Ok(synced)
}

pub async fn list_profiles_impl() -> Vec<ProfileEntry> {
    // Keep the active account's profile in step with Claude Code's token.
    let _ = sync_active_profile().await;
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
                    plan: plan_label(&token_data),
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
                    plan: String::new(),
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
    // Save the outgoing account's latest token before the keychain is
    // overwritten; it is the only valid copy once Claude Code has rotated it.
    if let Err(e) = sync_active_profile().await {
        return OpResult::err(&format!("Cannot save current account's token: {}", e));
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

    if let Err(e) = set_keychain_token(&pf.keychain_token).await {
        return OpResult::err(&e);
    }
    if let Err(e) = write_oauth_account(&pf.oauth_account) {
        return OpResult::err(&e);
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

/// POST a refresh_token grant to the OAuth token endpoint.
/// Retries with backoff on HTTP 429, honouring Retry-After when it is short.
async fn post_token_refresh(
    refresh_tok: &str,
    scopes: &[String],
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let mut body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": refresh_tok,
        "client_id": OAUTH_CLIENT_ID,
    });
    // Like Claude Code, ask for the scopes the token already has.
    if !scopes.is_empty() {
        body["scope"] = serde_json::Value::String(scopes.join(" "));
    }

    let mut attempt: u32 = 1;
    loop {
        let resp = client
            .post(OAUTH_TOKEN_URL)
            .header("Content-Type", "application/json")
            // Claude Code sends axios' default User-Agent here. Requests
            // claiming to be "claude-code/x.y.z" are answered with HTTP 429
            // regardless of how rarely they are made.
            .header("User-Agent", REFRESH_USER_AGENT)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        let status = resp.status().as_u16();
        if resp.status().is_success() {
            return resp
                .json()
                .await
                .map_err(|e| format!("Parse response: {}", e));
        }

        let retry_after = parse_retry_after(resp.headers());
        let body_text = resp.text().await.unwrap_or_default();
        if status != 429 {
            return Err(describe_http_error(status, &body_text));
        }

        // Rate limited: back off and retry while the wait is short.
        let wait_secs = retry_after.unwrap_or(1u64 << (attempt - 1));
        if attempt >= REFRESH_MAX_ATTEMPTS || wait_secs > REFRESH_MAX_WAIT_SECS {
            let hint = match retry_after {
                Some(s) => format!("請在 {} 秒後再試", s),
                None => "請稍後再試".to_string(),
            };
            return Err(format!("請求太頻繁（HTTP 429），{}", hint));
        }
        sleep(Duration::from_secs(wait_secs)).await;
        attempt += 1;
    }
}

/// Retry-After in seconds; an HTTP-date value is ignored.
fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// Turn a non-2xx token response into a one-line, human-readable error.
fn describe_http_error(status: u16, body: &str) -> String {
    let json: Option<serde_json::Value> = serde_json::from_str(body).ok();
    let err = json.as_ref().and_then(|v| v.get("error"));

    // Anthropic style: {"error": {"type": "...", "message": "..."}}
    // OAuth style:     {"error": "invalid_grant", "error_description": "..."}
    let code = err
        .and_then(|e| e.get("type").and_then(|t| t.as_str()).or_else(|| e.as_str()))
        .unwrap_or("");
    let detail = err
        .and_then(|e| e.get("message").and_then(|m| m.as_str()))
        .or_else(|| json.as_ref()?.get("error_description")?.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| body.trim().to_string());

    match code {
        "invalid_grant" => {
            "Refresh token 已失效，請在 Claude Code 重新登入後再備份此 profile".to_string()
        }
        _ if detail.is_empty() => format!("HTTP {}", status),
        _ => format!("HTTP {}: {}", status, detail),
    }
}

/// Refresh the access token of `profile_name` (or the active keychain token when None).
/// Unless `force` is set, a token that is still comfortably valid is left alone.
pub async fn refresh_token_impl(profile_name: Option<String>, force: bool) -> OpResult {
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
        (pf.keychain_token, Some(path))
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

    // Skip while the token is still comfortably valid, unless the caller insists.
    if !force {
        let expires_at = token_json
            .get("claudeAiOauth")
            .and_then(|o| o.get("expiresAt"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        if expires_at > now_ms() + REFRESH_SKIP_MARGIN_MS {
            let mut r = OpResult::ok("Token 仍有效，已略過刷新");
            r.expires_at = Some(format_local_time((expires_at / 1000) as i64));
            r.skipped = Some(true);
            return r;
        }
    }

    let scopes: Vec<String> = token_json
        .get("claudeAiOauth")
        .and_then(|o| o.get("scopes"))
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let resp_data = match post_token_refresh(&refresh_tok, &scopes).await {
        Ok(v) => v,
        Err(e) => return OpResult::err(&e),
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
    let mut r = OpResult::ok("Token 刷新完成");
    r.expires_at = Some(format_local_time((expires_at / 1000) as i64));
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
async fn refresh_token(profile_name: Option<String>, force: Option<bool>) -> OpResult {
    refresh_token_impl(profile_name, force.unwrap_or(false)).await
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// On HTTP 429: seconds the server asked us to wait (Retry-After). The
    /// usage endpoint's lockouts run from minutes to about an hour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_secs: Option<u64>,
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
                ..Default::default()
            };
        }
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
                    ..Default::default()
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
                    ..Default::default()
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
                ..Default::default()
            }
        }
    };

    // Call usage API
    let client = reqwest::Client::new();
    let resp = match client
        .get("https://api.anthropic.com/api/oauth/usage")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("User-Agent", REFRESH_USER_AGENT)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return UsageResult {
                success: false,
                error: Some(format!("HTTP error: {}", e)),
                data: None,
                ..Default::default()
            }
        }
    };

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let retry_after_secs = if status == 429 {
            parse_retry_after(resp.headers())
        } else {
            None
        };
        let body_text = resp.text().await.unwrap_or_default();
        return UsageResult {
            success: false,
            error: Some(describe_http_error(status, &body_text)),
            data: None,
            retry_after_secs,
        };
    }

    match resp.json::<serde_json::Value>().await {
        Ok(data) => UsageResult {
            success: true,
            error: None,
            data: Some(data),
            ..Default::default()
        },
        Err(e) => UsageResult {
            success: false,
            error: Some(format!("Parse response: {}", e)),
            data: None,
            ..Default::default()
        },
    }
}

#[tauri::command]
async fn get_usage(profile_name: Option<String>) -> UsageResult {
    get_usage_impl(profile_name).await
}

/// Format an epoch timestamp (seconds) as `YYYY-MM-DD HH:MM:SS` in the local time zone.
fn format_local_time(epoch_secs: i64) -> String {
    use chrono::{Local, TimeZone};
    match Local.timestamp_opt(epoch_secs, 0).single() {
        Some(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        None => format!("epoch {}", epoch_secs),
    }
}

// ── Menu bar (tray) ──

const TRAY_ID: &str = "main";
/// Passed by the login item: start in the menu bar without opening the window.
const HIDDEN_ARG: &str = "--hidden";

/// One account row in the tray menu; the frontend formats the label.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrayProfile {
    name: String,
    label: String,
    active: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrayState {
    /// Text next to the menu bar icon (macOS), e.g. "Sean 97%".
    title: String,
    tooltip: String,
    profiles: Vec<TrayProfile>,
}

fn build_tray_menu(app: &AppHandle, profiles: &[TrayProfile]) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    if profiles.is_empty() {
        menu.append(&MenuItem::with_id(app, "none", "No profiles yet", false, None::<&str>)?)?;
    }
    for p in profiles {
        // Clicking an account switches to it; the active one carries the check.
        menu.append(&CheckMenuItem::with_id(
            app,
            format!("switch:{}", p.name),
            &p.label,
            true,
            p.active,
            None::<&str>,
        )?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "refresh", "Refresh Usage", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "open", "Open C-Switcher", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    let at_login = app.autolaunch().is_enabled().unwrap_or(false);
    menu.append(&CheckMenuItem::with_id(
        app,
        "autostart",
        "Launch at Login",
        true,
        at_login,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quit C-Switcher", true, None::<&str>)?)?;
    Ok(menu)
}

/// Rebuild the tray menu and title from the frontend's current view.
/// Sync command, so it runs on the main thread as menu updates require.
#[tauri::command]
fn update_tray(app: AppHandle, state: TrayState) -> Result<(), String> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    let menu = build_tray_menu(&app, &state.profiles).map_err(|e| e.to_string())?;
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    tray.set_title(Some(state.title.as_str()))
        .map_err(|e| e.to_string())?;
    tray.set_tooltip(Some(state.tooltip.as_str()))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Show the window and, on macOS, the Dock icon that goes with it.
fn show_main_window(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(ActivationPolicy::Regular);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Hide the window and, on macOS, the Dock icon: only the menu bar item stays.
fn hide_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(ActivationPolicy::Accessory);
}

fn toggle_autostart(app: &AppHandle) {
    let launcher = app.autolaunch();
    let _ = if launcher.is_enabled().unwrap_or(false) {
        launcher.disable()
    } else {
        launcher.enable()
    };
    // Rebuild the menu so the check mark matches the real setting.
    let _ = app.emit("tray-changed", ());
}


fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .icon_as_template(true)
        .tooltip("C-Switcher")
        .menu(&build_tray_menu(app, &[])?)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "quit" => app.exit(0),
            "open" => show_main_window(app),
            "autostart" => toggle_autostart(app),
            "refresh" => {
                let _ = app.emit("tray-refresh", ());
            }
            id => {
                if let Some(name) = id.strip_prefix("switch:") {
                    let _ = app.emit("tray-switch", name.to_string());
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            // Opening the menu is a good moment to bring usage up to date.
            if let TrayIconEvent::Click { .. } = event {
                let _ = tray.app_handle().emit("tray-opened", ());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        // Must come first: a second launch (Dock, Finder, a stray login item)
        // just brings up the running instance instead of adding a second
        // menu bar icon.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if !args.iter().any(|a| a == HIDDEN_ARG) {
                show_main_window(app);
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_ARG]),
        ))
        .setup(|app| {
            create_tray(app.handle())?;
            // The window starts hidden (tauri.conf.json); open it unless this
            // is a login launch.
            if std::env::args().any(|a| a == HIDDEN_ARG) {
                hide_main_window(app.handle());
            } else {
                show_main_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window keeps the app in the menu bar; Quit is in
            // the tray menu (and Cmd+Q while the window is open).
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                hide_main_window(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_current_info,
            list_profiles,
            backup_profile,
            switch_profile,
            delete_profile,
            refresh_token,
            get_usage,
            update_tray,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| {
        // Dock icon clicked while the window is hidden: bring it back.
        #[cfg(target_os = "macos")]
        if let RunEvent::Reopen {
            has_visible_windows: false,
            ..
        } = event
        {
            show_main_window(app);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (app, event);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};

    #[test]
    fn describe_http_error_anthropic_style() {
        let body = r#"{ "error": { "type": "rate_limit_error", "message": "Rate limited. Please try again later." } }"#;
        assert_eq!(
            describe_http_error(429, body),
            "HTTP 429: Rate limited. Please try again later."
        );
    }

    #[test]
    fn describe_http_error_invalid_grant_is_translated() {
        let oauth = r#"{"error":"invalid_grant","error_description":"refresh token revoked"}"#;
        let anthropic = r#"{"error":{"type":"invalid_grant","message":"x"}}"#;
        for body in [oauth, anthropic] {
            assert!(describe_http_error(400, body).starts_with("Refresh token 已失效"));
        }
    }

    #[test]
    fn describe_http_error_falls_back_to_raw_or_status() {
        assert_eq!(describe_http_error(502, "<html>Bad Gateway</html>"), "HTTP 502: <html>Bad Gateway</html>");
        assert_eq!(describe_http_error(503, "   "), "HTTP 503");
    }

    #[test]
    fn plan_label_from_token() {
        let t = |sub: &str, tier: &str| {
            serde_json::json!({"claudeAiOauth": {"subscriptionType": sub, "rateLimitTier": tier}})
        };
        assert_eq!(plan_label(&t("max", "default_claude_max_20x")), "Max 20x");
        assert_eq!(plan_label(&t("max", "default_claude_max_5x")), "Max 5x");
        assert_eq!(plan_label(&t("pro", "default_claude_ai")), "Pro");
        assert_eq!(plan_label(&t("team", "")), "Team");
        assert_eq!(plan_label(&serde_json::json!({})), "");
    }

    #[test]
    fn parse_retry_after_seconds_only() {
        let mut h = HeaderMap::new();
        assert_eq!(parse_retry_after(&h), None);
        h.insert(RETRY_AFTER, HeaderValue::from_static(" 7 "));
        assert_eq!(parse_retry_after(&h), Some(7));
        h.insert(RETRY_AFTER, HeaderValue::from_static("Wed, 21 Oct 2015 07:28:00 GMT"));
        assert_eq!(parse_retry_after(&h), None);
    }
}
