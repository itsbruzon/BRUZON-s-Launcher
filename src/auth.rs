use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::time::{sleep, Duration};

const PRISM_MSA_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";
const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftAccount {
    pub id: String,
    pub name: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64,
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
    expires_in: Option<u64>,
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
struct MinecraftResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct MinecraftProfile {
    id: String,
    name: String,
}

pub async fn request_device_code() -> Result<DeviceCode> {
    let response = Client::new()
        .post(DEVICE_CODE_URL)
        .form(&[
            ("client_id", PRISM_MSA_CLIENT_ID),
            ("scope", "XboxLive.SignIn XboxLive.offline_access"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<DeviceCodeResponse>()
        .await?;

    Ok(DeviceCode {
        device_code: response.device_code,
        user_code: response.user_code,
        verification_uri: response.verification_uri,
        expires_in: response.expires_in,
        interval: response.interval.unwrap_or(5).max(1),
    })
}

pub async fn complete_device_login(device: DeviceCode) -> Result<MinecraftAccount> {
    let client = Client::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(device.expires_in);
    let mut interval = device.interval;

    let msa_token = loop {
        if std::time::Instant::now() >= deadline {
            return Err(anyhow!("Microsoft device-code login expired"));
        }

        sleep(Duration::from_secs(interval)).await;
        let token = client
            .post(TOKEN_URL)
            .form(&[
                ("client_id", PRISM_MSA_CLIENT_ID),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", device.device_code.as_str()),
            ])
            .send()
            .await?
            .json::<TokenResponse>()
            .await?;

        if let Some(access_token) = token.access_token {
            break (access_token, token.refresh_token);
        }

        match token.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                interval += 5;
                continue;
            }
            Some(error) => {
                return Err(anyhow!(
                    "Microsoft login failed: {}",
                    token.error_description.unwrap_or_else(|| error.to_string())
                ));
            }
            None => return Err(anyhow!("Microsoft returned an invalid token response")),
        }
    };

    let xbox = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", msa_token.0),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<XboxResponse>()
        .await?;

    let uhs = xbox.display_claims.xui.first().map(|claim| claim.uhs.clone())
        .ok_or_else(|| anyhow!("Xbox Live did not return a user hash"))?;

    let xsts = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("x-xbl-contract-version", "1")
        .json(&serde_json::json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbox.token] },
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
            "identityToken": format!("XBL3.0 x={};{}", uhs, xsts.token)
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<MinecraftResponse>()
        .await?;

    let profile = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&minecraft.access_token)
        .send()
        .await?
        .error_for_status()?
        .json::<MinecraftProfile>()
        .await?;

    Ok(MinecraftAccount {
        id: profile.id,
        name: profile.name,
        access_token: minecraft.access_token,
        refresh_token: msa_token.1,
        expires_at: now_unix() + minecraft.expires_in,
    })
}

pub async fn save_accounts(path: &Path, accounts: &[MinecraftAccount]) -> Result<()> {
    if let Some(parent) = path.parent() { tokio::fs::create_dir_all(parent).await?; }
    tokio::fs::write(path, serde_json::to_vec_pretty(accounts)?).await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).await?;
    }
    Ok(())
}

pub async fn load_accounts(path: &Path) -> Result<Vec<MinecraftAccount>> {
    if !tokio::fs::try_exists(path).await.unwrap_or(false) { return Ok(Vec::new()); }
    Ok(serde_json::from_slice(&tokio::fs::read(path).await?).unwrap_or_default())
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0)
}
