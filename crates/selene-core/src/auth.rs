//! Sign-in with a Microsoft account, as the official launcher does it:
//! Microsoft OAuth, then Xbox Live, then XSTS, then Minecraft Services.

use std::time::{Duration, SystemTime};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use url::Url;

use crate::{Error, Result};

pub const CLIENT_ID: &str = "e193bac3-0149-4777-bd09-91afa62cd0f3";
const CALLBACK_READ_TIMEOUT: Duration = Duration::from_secs(5);
const SERVICE_RETRIES: u32 = 3;
const SERVICE_MAX_WAIT: Duration = Duration::from_secs(8);

const AUTHORIZE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const DEVICE_CODE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
const XBOX_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MINECRAFT_LOGIN_URL: &str =
    "https://api.minecraftservices.com/authentication/login_with_xbox";
pub(crate) const MINECRAFT_PROFILE_URL: &str =
    "https://api.minecraftservices.com/minecraft/profile";
const SCOPE: &str = "XboxLive.signin offline_access";

const CALLBACK_PAGE: &str = "<!doctype html><meta charset=utf-8><title>Selene Launcher</title>\
<body style=\"font-family:system-ui;text-align:center;padding-top:20vh\">\
<h2>Готово / Done</h2><p>Можно закрыть эту вкладку и вернуться в лаунчер.<br>\
You can close this tab and return to the launcher.</p>";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("sign-in was cancelled or denied: {0}")]
    Denied(String),
    #[error("the sign-in response did not match the request")]
    InvalidCallback,
    #[error("this Microsoft account has no Xbox profile yet")]
    NoXboxAccount,
    #[error("Xbox Live is not available in this account's country")]
    RegionBlocked,
    #[error("a child account must be added to a family by an adult first")]
    ChildAccount,
    #[error("this account does not own Minecraft: Java Edition")]
    GameNotOwned,
    #[error("the saved sign-in has expired or was revoked")]
    SessionExpired,
    #[error("Minecraft Services asked to slow down")]
    RateLimited,
    #[error("{service} returned {status}")]
    Service {
        service: &'static str,
        status: StatusCode,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftSession {
    pub access_token: String,
    pub expires_at: SystemTime,
    pub profile: Profile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<Skin>,
    #[serde(default)]
    pub capes: Vec<Cape>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skin {
    pub id: String,
    pub state: String,
    #[serde(deserialize_with = "secure_url")]
    pub url: String,
    pub variant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cape {
    pub id: String,
    pub state: String,
    #[serde(deserialize_with = "secure_url")]
    pub url: String,
    pub alias: String,
}

/// A sign-in waiting for the user to finish in their browser.
pub struct LoginRequest {
    pub url: Url,
    listener: TcpListener,
    redirect_uri: String,
    verifier: String,
    state: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxResponse {
    token: String,
    display_claims: XboxClaims,
}

#[derive(Deserialize)]
struct XboxClaims {
    xui: Vec<XboxUser>,
}

#[derive(Deserialize)]
struct XboxUser {
    uhs: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XstsError {
    x_err: u64,
}

#[derive(Deserialize)]
struct MinecraftLoginResponse {
    access_token: String,
    expires_in: u64,
}

impl LoginRequest {
    /// Opens a local port for Microsoft to redirect back to and builds the URL to show the user.
    ///
    /// # Panics
    ///
    /// Panics if the operating system cannot provide random bytes.
    pub async fn start() -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|err| Error::io("127.0.0.1", err))?;
        let port = listener
            .local_addr()
            .map_err(|err| Error::io("127.0.0.1", err))?
            .port();
        let redirect_uri = format!("http://localhost:{port}");
        let verifier = random_token();
        let state = random_token();

        let url = Url::parse_with_params(
            AUTHORIZE_URL,
            [
                ("client_id", CLIENT_ID),
                ("response_type", "code"),
                ("redirect_uri", &redirect_uri),
                ("scope", SCOPE),
                ("code_challenge", &pkce_challenge(&verifier)),
                ("code_challenge_method", "S256"),
                ("state", &state),
                ("prompt", "select_account"),
            ],
        )
        .expect("authorize url is valid");

        Ok(Self {
            url,
            listener,
            redirect_uri,
            verifier,
            state,
        })
    }

    /// Waits for the browser to come back and exchanges the code for Microsoft tokens.
    pub async fn finish(self, client: &Client) -> Result<MicrosoftTokens> {
        let code = loop {
            let (mut stream, _) = self
                .listener
                .accept()
                .await
                .map_err(|err| Error::io("127.0.0.1", err))?;
            // Browsers open spare connections that never send anything; waiting on one of
            // them would hold up the real redirect behind it.
            let mut request_line = String::new();
            let mut reader = BufReader::new(&mut stream);
            let read = reader.read_line(&mut request_line);
            if !matches!(
                tokio::time::timeout(CALLBACK_READ_TIMEOUT, read).await,
                Ok(Ok(_))
            ) {
                continue;
            }

            // Browsers also ask for /favicon.ico; only the request carrying our state counts.
            let Some(result) = parse_callback(&request_line, &self.state) else {
                let _ = stream
                    .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
                    .await;
                continue;
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{CALLBACK_PAGE}",
                CALLBACK_PAGE.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            break result?;
        };

        request_tokens(
            client,
            &[
                ("client_id", CLIENT_ID),
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", &self.redirect_uri),
                ("code_verifier", &self.verifier),
                ("scope", SCOPE),
            ],
        )
        .await
    }
}

/// Sign-in on another device: the user opens `verification_uri` and types `user_code`.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceLogin {
    pub user_code: String,
    pub verification_uri: String,
    device_code: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct OAuthError {
    error: String,
    error_description: Option<String>,
}

impl DeviceLogin {
    pub async fn request(client: &Client) -> Result<Self> {
        let response = client
            .post(DEVICE_CODE_URL)
            .form(&[("client_id", CLIENT_ID), ("scope", SCOPE)])
            .send()
            .await?;
        parse_response(response, "Microsoft").await
    }

    /// Polls until the user finishes, gives up, or the code expires.
    pub async fn finish(self, client: &Client) -> Result<MicrosoftTokens> {
        let deadline = SystemTime::now() + Duration::from_secs(self.expires_in);
        let mut interval = Duration::from_secs(self.interval.max(1));

        while SystemTime::now() < deadline {
            tokio::time::sleep(interval).await;
            let response = client
                .post(TOKEN_URL)
                .form(&[
                    ("client_id", CLIENT_ID),
                    ("grant_type", DEVICE_CODE_GRANT),
                    ("device_code", &self.device_code),
                ])
                .send()
                .await?;
            if response.status().is_success() {
                let tokens: TokenResponse = response.json().await?;
                return Ok(tokens.into());
            }

            let error: OAuthError = response.json().await?;
            match error.error.as_str() {
                "authorization_pending" => {}
                "slow_down" => interval += Duration::from_secs(5),
                _ => {
                    return Err(
                        AuthError::Denied(error.error_description.unwrap_or(error.error)).into(),
                    );
                }
            }
        }
        Err(AuthError::Denied("the code expired".into()).into())
    }
}

pub async fn refresh(client: &Client, refresh_token: &str) -> Result<MicrosoftTokens> {
    request_tokens(
        client,
        &[
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ],
    )
    .await
    .map_err(|err| match err {
        // Microsoft answers 400 `invalid_grant` once a refresh token has expired or been revoked.
        Error::Auth(AuthError::Service {
            status: StatusCode::BAD_REQUEST,
            ..
        }) => AuthError::SessionExpired.into(),
        other => other,
    })
}

/// Trades a Microsoft access token for a Minecraft session and checks that the game is owned.
pub async fn minecraft_login(
    client: &Client,
    microsoft_access_token: &str,
) -> Result<MinecraftSession> {
    let xbox: XboxResponse = post_json(
        client,
        "Xbox Live",
        XBOX_AUTH_URL,
        &json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={microsoft_access_token}"),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }),
    )
    .await?;

    let xsts = xsts_authorize(client, &xbox.token).await?;
    let user_hash = xsts
        .display_claims
        .xui
        .first()
        .map(|user| user.uhs.as_str())
        .ok_or(AuthError::NoXboxAccount)?;

    let login: MinecraftLoginResponse = post_json(
        client,
        "Minecraft Services",
        MINECRAFT_LOGIN_URL,
        &json!({ "identityToken": format!("XBL3.0 x={user_hash};{}", xsts.token) }),
    )
    .await?;

    let profile = fetch_profile(client, &login.access_token).await?;
    Ok(MinecraftSession {
        access_token: login.access_token,
        expires_at: expires_at(login.expires_in),
        profile,
    })
}

/// A missing profile means the account has never owned the game.
pub async fn fetch_profile(client: &Client, minecraft_access_token: &str) -> Result<Profile> {
    let response = send_to_services(|| {
        Ok(client
            .get(MINECRAFT_PROFILE_URL)
            .bearer_auth(minecraft_access_token))
    })
    .await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Err(AuthError::GameNotOwned.into());
    }
    parse_response(response, "Minecraft profile").await
}

async fn xsts_authorize(client: &Client, xbox_token: &str) -> Result<XboxResponse> {
    let response = client
        .post(XSTS_AUTH_URL)
        .json(&json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbox_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }))
        .send()
        .await?;

    if response.status() == StatusCode::UNAUTHORIZED {
        let error: XstsError = response.json().await?;
        return Err(xsts_error(error.x_err).into());
    }
    parse_response(response, "XSTS").await
}

/// Mojang hands out texture links over plain HTTP, which the window refuses to load.
fn secure_url<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<String, D::Error> {
    let url = String::deserialize(deserializer)?;
    Ok(match url.strip_prefix("http://") {
        Some(rest) => format!("https://{rest}"),
        None => url,
    })
}

fn xsts_error(code: u64) -> AuthError {
    match code {
        2_148_916_233 => AuthError::NoXboxAccount,
        2_148_916_235 => AuthError::RegionBlocked,
        2_148_916_236..=2_148_916_238 => AuthError::ChildAccount,
        _ => AuthError::Service {
            service: "XSTS",
            status: StatusCode::UNAUTHORIZED,
        },
    }
}

async fn request_tokens(client: &Client, form: &[(&str, &str)]) -> Result<MicrosoftTokens> {
    let response = client.post(TOKEN_URL).form(form).send().await?;
    let tokens: TokenResponse = parse_response(response, "Microsoft").await?;
    Ok(tokens.into())
}

impl From<TokenResponse> for MicrosoftTokens {
    fn from(tokens: TokenResponse) -> Self {
        Self {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_at: expires_at(tokens.expires_in),
        }
    }
}

async fn post_json<T: DeserializeOwned>(
    client: &Client,
    service: &'static str,
    url: &str,
    body: &serde_json::Value,
) -> Result<T> {
    let response = client.post(url).json(body).send().await?;
    parse_response(response, service).await
}

pub(crate) async fn parse_response<T: DeserializeOwned>(
    response: Response,
    service: &'static str,
) -> Result<T> {
    let status = response.status();
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(AuthError::RateLimited.into());
    }
    if !status.is_success() {
        return Err(AuthError::Service { service, status }.into());
    }
    Ok(response.json().await?)
}

/// Minecraft Services answers a burst of requests with 429 and now and then fails with a
/// 5xx. Such answers are tried again a few times, waiting as long as the server asks.
pub(crate) async fn send_to_services(
    request: impl Fn() -> Result<RequestBuilder>,
) -> Result<Response> {
    let mut attempt = 0;
    loop {
        let response = request()?.send().await?;
        let status = response.status();
        let busy = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
        if !busy || attempt == SERVICE_RETRIES {
            return Ok(response);
        }
        attempt += 1;
        let wait = retry_after(&response)
            .unwrap_or(Duration::from_secs(1 << attempt))
            .min(SERVICE_MAX_WAIT);
        tokio::time::sleep(wait).await;
    }
}

fn retry_after(response: &Response) -> Option<Duration> {
    let seconds = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(Duration::from_secs(seconds))
}

/// Pulls the authorization code out of the redirect's request line, e.g.
/// `GET /?code=...&state=... HTTP/1.1`. Returns `None` for unrelated requests.
fn parse_callback(request_line: &str, expected_state: &str) -> Option<Result<String>> {
    let target = request_line.split_whitespace().nth(1)?;
    let url = Url::parse("http://localhost").ok()?.join(target).ok()?;
    let param = |name: &str| {
        url.query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };

    // Anything else on the machine can reach this port, so a request without our state is
    // ignored instead of ending the sign-in.
    if param("state")? != expected_state {
        return None;
    }
    if let Some(error) = param("error") {
        let description = param("error_description").unwrap_or(error);
        return Some(Err(AuthError::Denied(description).into()));
    }
    Some(param("code").ok_or_else(|| AuthError::InvalidCallback.into()))
}

fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("system random generator is available");
    URL_SAFE_NO_PAD.encode(bytes)
}

fn expires_at(expires_in: u64) -> SystemTime {
    SystemTime::now() + Duration::from_secs(expires_in)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_textures_over_https() {
        let skin: Skin = serde_json::from_value(serde_json::json!({
            "id": "a",
            "state": "ACTIVE",
            "url": "http://textures.minecraft.net/texture/abc",
            "variant": "CLASSIC",
        }))
        .unwrap();
        assert_eq!(skin.url, "https://textures.minecraft.net/texture/abc");
    }

    #[test]
    fn pkce_matches_rfc_7636_example() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn random_tokens_differ() {
        assert_ne!(random_token(), random_token());
        assert_eq!(random_token().len(), 43);
    }

    #[test]
    fn reads_code_from_callback() {
        let code = parse_callback("GET /?code=M.C5_abc&state=s1 HTTP/1.1\r\n", "s1");
        assert_eq!(code.unwrap().unwrap(), "M.C5_abc");
    }

    #[test]
    fn rejects_foreign_state_and_errors() {
        assert!(parse_callback("GET /favicon.ico HTTP/1.1", "s1").is_none());
        assert!(parse_callback("GET /?code=x&state=other HTTP/1.1", "s1").is_none());
        assert!(matches!(
            parse_callback(
                "GET /?error=access_denied&error_description=User%20cancelled&state=s1 HTTP/1.1",
                "s1"
            ),
            Some(Err(Error::Auth(AuthError::Denied(reason)))) if reason == "User cancelled"
        ));
    }

    #[test]
    fn maps_xsts_errors() {
        assert!(matches!(
            xsts_error(2_148_916_233),
            AuthError::NoXboxAccount
        ));
        assert!(matches!(xsts_error(2_148_916_238), AuthError::ChildAccount));
    }

    #[tokio::test]
    async fn start_builds_authorize_url() {
        let request = LoginRequest::start().await.unwrap();
        let params: Vec<_> = request.url.query_pairs().into_owned().collect();
        let param = |name: &str| {
            params
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.as_str())
        };

        assert_eq!(param("client_id"), Some(CLIENT_ID));
        assert_eq!(param("code_challenge_method"), Some("S256"));
        assert!(
            param("redirect_uri")
                .unwrap()
                .starts_with("http://localhost:")
        );
    }

    #[tokio::test]
    async fn retries_when_services_are_busy() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        tokio::spawn(async move {
            let answers = [
                "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 0\r\nContent-Length: 0\r\n\r\n",
                "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n",
            ];
            for answer in answers {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = [0; 1024];
                let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut request).await;
                stream.write_all(answer.as_bytes()).await.unwrap();
                stream.shutdown().await.unwrap();
            }
        });

        let client = Client::new();
        let response = send_to_services(|| Ok(client.get(&url))).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
