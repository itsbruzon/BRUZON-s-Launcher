use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tokio::time::{sleep, Duration};

const PRISM_MSA_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";
const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccountType {
    Microsoft,
    Offline,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Microsoft
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftAccount {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub account_type: AccountType,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_at: u64,
    #[serde(default)]
    pub skin_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct XboxResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Debug, Deserialize)]
struct DisplayClaims {
    xui: Vec<XuiClaim>,
}

#[derive(Debug, Deserialize)]
struct XuiClaim {
    uhs: String,
}

#[derive(Debug, Deserialize)]
struct MinecraftLoginResponse {
    access_token: String,
    expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct MinecraftEntitlements {
    #[serde(default)]
    items: Vec<MinecraftEntitlement>,
}

#[derive(Debug, Deserialize)]
struct MinecraftEntitlement {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MinecraftProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<MinecraftSkin>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MinecraftSkin {
    pub url: String,
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn offline_account(name: String) -> MinecraftAccount {
    let normalized = name.trim().to_string();
    MinecraftAccount {
        id: format!("offline-{}", normalized),
        name: normalized,
        account_type: AccountType::Offline,
        access_token: String::new(),
        refresh_token: None,
        expires_at: 0,
        skin_url: None,
    }
}

pub async fn request_device_code() -> Result<DeviceCode> {
    let client = Client::new();
    let response = client
        .post(DEVICE_CODE_URL)
        .form(&[
            ("client_id", PRISM_MSA_CLIENT_ID),
            ("scope", "XboxLive.SignIn XboxLive.offline_access"),
        ])
        .send()
        .await?
        .error_for_status()?;

    let data: DeviceCodeResponse = response.json().await?;

    Ok(DeviceCode {
        device_code: data.device_code,
        user_code: data.user_code,
        verification_uri: data.verification_uri,
        expires_in: data.expires_in,
        interval: data.interval.unwrap_or(5),
    })
}

async fn poll_microsoft_token(device: &DeviceCode) -> Result<TokenResponse> {
    let client = Client::new();
    let deadline = now_unix().saturating_add(device.expires_in);
    let mut interval = device.interval.max(1);

    loop {
        if now_unix() >= deadline {
            return Err(anyhow!("Microsoft sign-in timed out. Please try again."));
        }

        let response = client
            .post(TOKEN_URL)
            .form(&[
                ("client_id", PRISM_MSA_CLIENT_ID),
                (
                    "grant_type",
                    "urn:ietf:params:oauth:grant-type:device_code",
                ),
                ("device_code", device.device_code.as_str()),
            ])
            .send()
            .await?;

        let status = response.status();
        let token: TokenResponse = response.json().await?;

        if status.is_success() {
            return Ok(token);
        }

        match token.error.as_deref() {
            Some("authorization_pending") => sleep(Duration::from_secs(interval)).await,
            Some("slow_down") => {
                interval = interval.saturating_add(5);
                sleep(Duration::from_secs(interval)).await;
            }
            Some(error) => {
                return Err(anyhow!(
                    "Microsoft sign-in failed: {}",
                    token.error_description.as_deref().unwrap_or(error)
                ));
            }
            None => return Err(anyhow!("Microsoft sign-in failed: HTTP {}", status)),
        }
    }
}

pub async fn complete_device_login(device: DeviceCode) -> Result<MinecraftAccount> {
    let microsoft = poll_microsoft_token(&device).await?;
    let microsoft_access_token = microsoft
        .access_token
        .ok_or_else(|| anyhow!("Microsoft did not return an access token"))?;

    let client = Client::new();

    let xbox_response = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", microsoft_access_token)
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<XboxResponse>()
        .await?;

    let uhs = xbox_response
        .display_claims
        .xui
        .first()
        .map(|claim| claim.uhs.clone())
        .ok_or_else(|| anyhow!("Xbox Live did not return a user hash"))?;

    let xsts_response = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbox_response.token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<XboxResponse>()
        .await?;

    let minecraft = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "identityToken": format!("XBL3.0 x={};{}", uhs, xsts_response.token)
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<MinecraftLoginResponse>()
        .await?;

    let entitlements_response = client
        .get("https://api.minecraftservices.com/entitlements/mcstore")
        .bearer_auth(&minecraft.access_token)
        .send()
        .await?;
    let entitlements = entitlements_response.error_for_status()?.json::<MinecraftEntitlements>().await?;

    let owns_minecraft = entitlements.items.iter().any(|item| {
        matches!(item.name.as_str(), "product_minecraft" | "game_minecraft")
    });

    if !owns_minecraft {
        let items = entitlements
            .items
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(anyhow!(
            "Minecraft entitlement check returned no Java entitlement (items: {}).",
            if items.is_empty() { "none" } else { &items }
        ));
    }

    let profile_response = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&minecraft.access_token)
        .send()
        .await?;

    if profile_response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow!(
            "Minecraft profile was not found for this account. The account may own Minecraft but not have a Java profile yet."
        ));
    }

    let profile = profile_response
        .error_for_status()?
        .json::<MinecraftProfile>()
        .await?;

    Ok(MinecraftAccount {
        id: profile.id.clone(),
        name: profile.name,
        account_type: AccountType::Microsoft,
        access_token: minecraft.access_token,
        refresh_token: microsoft.refresh_token,
        expires_at: now_unix().saturating_add(minecraft.expires_in.unwrap_or(3600)),
        skin_url: profile.skins.first().map(|skin| skin.url.clone()),
    })
}

pub async fn save_accounts(path: &Path, accounts: &[MinecraftAccount]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let data = serde_json::to_vec_pretty(accounts)?;
    fs::write(path, data).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).await?;
    }
    Ok(())
}

pub async fn load_accounts(path: &Path) -> Result<Vec<MinecraftAccount>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read(path).await?;
    Ok(serde_json::from_slice(&data)?)
}

pub async fn save_selected_account(path: &Path, account_id: Option<&str>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    match account_id {
        Some(id) => fs::write(path, id).await?,
        None => {
            if path.exists() {
                fs::remove_file(path).await?;
            }
        }
    }
    Ok(())
}

pub async fn load_selected_account(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let id = fs::read_to_string(path).await?;
    let id = id.trim();
    if id.is_empty() {
        Ok(None)
    } else {
        Ok(Some(id.to_string()))
    }
}
