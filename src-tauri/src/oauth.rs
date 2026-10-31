//! Installed-app Google and Microsoft OAuth. Only account metadata crosses the WebView boundary.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use url::Url;

const MAIL_SCOPE: &str = "https://mail.google.com/";
const SERVICE: &str = "com.imapsync.studio.google-oauth";
pub const MICROSOFT_SCOPE: &str = "https://outlook.office.com/IMAP.AccessAsUser.All";
const MICROSOFT_AUTH_SCOPE: &str =
    "https://outlook.office.com/IMAP.AccessAsUser.All offline_access openid profile email";
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Provider {
    #[default]
    Google,
    Microsoft,
}
impl Provider {
    fn scope(self) -> &'static str {
        if self == Self::Google {
            MAIL_SCOPE
        } else {
            MICROSOFT_SCOPE
        }
    }
    pub fn error(self, error: String) -> String {
        if self == Self::Microsoft {
            error.replace("GMAIL_OAUTH_", "MICROSOFT_OAUTH_")
        } else {
            error
        }
    }
}
fn valid_guid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

#[derive(Clone, Default)]
struct Config {
    client_id: String,
    client_secret: String,
    remember: bool,
    secret_persistent: bool,
}
#[derive(Clone, Serialize, Deserialize)]
struct Grant {
    email: String,
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    #[serde(default)]
    invalid: bool,
    #[serde(skip)]
    persistent: bool,
}
struct Attempt {
    id: String,
    status: String,
    email: String,
    error: String,
    cancel: Arc<AtomicBool>,
}
#[derive(Default)]
struct State {
    config: Config,
    accounts: HashMap<String, Grant>,
    attempt: Option<Attempt>,
}

trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<String, String>;
    fn put(&self, key: &str, value: &str) -> Result<(), String>;
    fn delete(&self, key: &str) -> Result<(), String>;
}
struct Keychain(&'static str);
impl SecretStore for Keychain {
    fn get(&self, key: &str) -> Result<String, String> {
        keyring::Entry::new(self.0, key)
            .and_then(|e| e.get_password())
            .map_err(|_| "GMAIL_OAUTH_STORAGE".into())
    }
    fn put(&self, key: &str, value: &str) -> Result<(), String> {
        keyring::Entry::new(self.0, key)
            .and_then(|e| e.set_password(value))
            .map_err(|_| "GMAIL_OAUTH_STORAGE".into())
    }
    fn delete(&self, key: &str) -> Result<(), String> {
        match keyring::Entry::new(self.0, key).and_then(|e| e.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("GMAIL_OAUTH_STORAGE".into()),
        }
    }
}

trait OAuthApi: Send + Sync {
    fn exchange(
        &self,
        config: &Config,
        code: &str,
        redirect: &str,
        verifier: &str,
    ) -> Result<Value, String>;
    fn refresh(&self, config: &Config, refresh: &str) -> Result<Value, String>;
    fn identity(&self, access: &str) -> Result<String, String>;
    fn identity_response(
        &self,
        data: &Value,
        _config: &Config,
        _nonce: &str,
    ) -> Result<String, String> {
        self.identity(
            data["access_token"]
                .as_str()
                .ok_or("GMAIL_OAUTH_RESPONSE")?,
        )
    }
}
struct Google;
fn http() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "GMAIL_OAUTH_NETWORK".into())
}
fn response_json(response: reqwest::blocking::Response) -> Result<Value, String> {
    let success = response.status().is_success();
    let mut bytes = Vec::new();
    response
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| "GMAIL_OAUTH_NETWORK")?;
    if bytes.len() > 65536 {
        return Err("GMAIL_OAUTH_RESPONSE".into());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "GMAIL_OAUTH_RESPONSE")?;
    if !success {
        return Err(if matches!(
            value["error"].as_str(),
            Some("invalid_grant" | "interaction_required" | "login_required" | "consent_required")
        ) {
            "GMAIL_OAUTH_RECONNECT"
        } else {
            "GMAIL_OAUTH_DENIED"
        }
        .into());
    }
    Ok(value)
}
impl OAuthApi for Google {
    fn exchange(
        &self,
        c: &Config,
        code: &str,
        redirect: &str,
        verifier: &str,
    ) -> Result<Value, String> {
        let mut form = vec![
            ("client_id", c.client_id.as_str()),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect),
            ("grant_type", "authorization_code"),
        ];
        if !c.client_secret.is_empty() {
            form.push(("client_secret", &c.client_secret));
        }
        response_json(
            http()?
                .post("https://oauth2.googleapis.com/token")
                .form(&form)
                .send()
                .map_err(|_| "GMAIL_OAUTH_NETWORK")?,
        )
    }
    fn refresh(&self, c: &Config, refresh: &str) -> Result<Value, String> {
        let mut form = vec![
            ("client_id", c.client_id.as_str()),
            ("refresh_token", refresh),
            ("grant_type", "refresh_token"),
        ];
        if !c.client_secret.is_empty() {
            form.push(("client_secret", &c.client_secret));
        }
        response_json(
            http()?
                .post("https://oauth2.googleapis.com/token")
                .form(&form)
                .send()
                .map_err(|_| "GMAIL_OAUTH_NETWORK")?,
        )
    }
    fn identity(&self, access: &str) -> Result<String, String> {
        let data = response_json(
            http()?
                .get("https://openidconnect.googleapis.com/v1/userinfo")
                .bearer_auth(access)
                .send()
                .map_err(|_| "GMAIL_OAUTH_NETWORK")?,
        )?;
        let email = data["email"].as_str().unwrap_or("").trim().to_lowercase();
        if data["email_verified"] != true || !valid_email(&email) {
            return Err("GMAIL_OAUTH_IDENTITY".into());
        }
        Ok(email)
    }
}

fn authorization_url(
    provider: Provider,
    client: &str,
    redirect: &str,
    state: &str,
    verifier: &str,
    nonce: &str,
) -> Url {
    let mut url = Url::parse(if provider == Provider::Google {
        "https://accounts.google.com/o/oauth2/v2/auth"
    } else {
        "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
    })
    .unwrap();
    let scope = if provider == Provider::Google {
        format!("{MAIL_SCOPE} openid email")
    } else {
        MICROSOFT_AUTH_SCOPE.to_string()
    };
    url.query_pairs_mut().extend_pairs([
        ("client_id", client),
        ("redirect_uri", redirect),
        ("response_type", "code"),
        ("scope", scope.as_str()),
        ("state", state),
        ("code_challenge", challenge(verifier).as_str()),
        ("code_challenge_method", "S256"),
    ]);
    if provider == Provider::Google {
        url.query_pairs_mut().extend_pairs([
            ("access_type", "offline"),
            ("prompt", "consent select_account"),
        ]);
    } else {
        url.query_pairs_mut().extend_pairs([
            ("nonce", nonce),
            ("prompt", "select_account"),
            ("response_mode", "query"),
        ]);
    }
    url
}
struct Microsoft;
impl OAuthApi for Microsoft {
    fn exchange(
        &self,
        c: &Config,
        code: &str,
        redirect: &str,
        verifier: &str,
    ) -> Result<Value, String> {
        response_json(
            http()?
                .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
                .form(&[
                    ("client_id", c.client_id.as_str()),
                    ("code", code),
                    ("code_verifier", verifier),
                    ("redirect_uri", redirect),
                    ("grant_type", "authorization_code"),
                    ("scope", MICROSOFT_AUTH_SCOPE),
                ])
                .send()
                .map_err(|_| "GMAIL_OAUTH_NETWORK")?,
        )
    }
    fn refresh(&self, c: &Config, refresh: &str) -> Result<Value, String> {
        response_json(
            http()?
                .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
                .form(&[
                    ("client_id", c.client_id.as_str()),
                    ("refresh_token", refresh),
                    ("grant_type", "refresh_token"),
                    ("scope", MICROSOFT_SCOPE),
                ])
                .send()
                .map_err(|_| "GMAIL_OAUTH_NETWORK")?,
        )
    }
    fn identity(&self, _access: &str) -> Result<String, String> {
        Err("GMAIL_OAUTH_IDENTITY".into())
    }
    fn identity_response(
        &self,
        data: &Value,
        config: &Config,
        nonce: &str,
    ) -> Result<String, String> {
        let keys = response_json(
            http()?
                .get("https://login.microsoftonline.com/common/discovery/v2.0/keys")
                .send()
                .map_err(|_| "GMAIL_OAUTH_NETWORK")?,
        )?;
        microsoft_identity(
            data["id_token"].as_str().ok_or("GMAIL_OAUTH_IDENTITY")?,
            &config.client_id,
            nonce,
            &keys,
            chrono::Utc::now().timestamp(),
        )
    }
}
fn microsoft_identity(
    token: &str,
    client: &str,
    nonce: &str,
    keys: &Value,
    now: i64,
) -> Result<String, String> {
    let invalid = || "GMAIL_OAUTH_IDENTITY".to_string();
    if token.len() > 32768 || !valid_guid(client) || nonce.is_empty() {
        return Err(invalid());
    }
    let parts = token.split('.').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(invalid());
    }
    let decode = |part: &str| -> Result<Value, String> {
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(part).map_err(|_| invalid())?)
            .map_err(|_| invalid())
    };
    let header = decode(parts[0])?;
    let claims = decode(parts[1])?;
    let kid = header["kid"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 128)
        .ok_or_else(invalid)?;
    if header["alg"] != "RS256"
        || header.get("crit").is_some()
        || claims["aud"] != client
        || claims["nonce"] != nonce
    {
        return Err(invalid());
    }
    let tenant = claims["tid"]
        .as_str()
        .filter(|s| valid_guid(s))
        .ok_or_else(invalid)?;
    let issuer = format!("https://login.microsoftonline.com/{tenant}/v2.0");
    let exp = claims["exp"].as_i64().ok_or_else(invalid)?;
    let iat = claims["iat"].as_i64().ok_or_else(invalid)?;
    if claims["iss"] != issuer
        || exp < now.saturating_sub(60)
        || exp > now.saturating_add(86400)
        || iat > now.saturating_add(60)
        || iat < now.saturating_sub(86400)
        || exp <= iat
        || claims["nbf"]
            .as_i64()
            .is_some_and(|n| n > now.saturating_add(60))
    {
        return Err(invalid());
    }
    let signature = URL_SAFE_NO_PAD.decode(parts[2]).map_err(|_| invalid())?;
    let message = format!("{}.{}", parts[0], parts[1]);
    let verified = keys["keys"]
        .as_array()
        .ok_or_else(invalid)?
        .iter()
        .filter(|key| {
            key["kid"] == kid
                && key["kty"] == "RSA"
                && key["use"] == "sig"
                && (key.get("alg").is_none() || key["alg"] == "RS256")
                && key["issuer"]
                    .as_str()
                    .is_none_or(|s| s.replace("{tenantid}", tenant) == issuer)
        })
        .any(|key| {
            let decode = |name: &str| URL_SAFE_NO_PAD.decode(key[name].as_str().unwrap_or(""));
            let (Ok(n), Ok(e)) = (decode("n"), decode("e")) else {
                return false;
            };
            if !(256..=1024).contains(&n.len()) || e.len() > 8 {
                return false;
            }
            ring::signature::RsaPublicKeyComponents { n: &n, e: &e }
                .verify(
                    &ring::signature::RSA_PKCS1_2048_8192_SHA256,
                    message.as_bytes(),
                    &signature,
                )
                .is_ok()
        });
    if !verified {
        return Err(invalid());
    }
    let email = ["preferred_username", "email"]
        .iter()
        .filter_map(|key| claims[*key].as_str())
        .map(|s| s.trim().to_lowercase())
        .find(|s| valid_email(s))
        .ok_or_else(invalid)?;
    Ok(email)
}

pub struct Manager {
    provider: Provider,
    state: Mutex<State>,
    path: Option<PathBuf>,
    store: Arc<dyn SecretStore>,
    api: Arc<dyn OAuthApi>,
}
impl Default for Manager {
    fn default() -> Self {
        Self {
            provider: Provider::Google,
            state: Mutex::new(State::default()),
            path: None,
            store: Arc::new(Keychain(SERVICE)),
            api: Arc::new(Google),
        }
    }
}
fn key(client: &str, email: &str) -> String {
    format!(
        "{}:{email}",
        URL_SAFE_NO_PAD.encode(Sha256::digest(client.as_bytes()))
    )
}
fn valid_email(email: &str) -> bool {
    email.len() <= 320
        && email.contains('@')
        && !email.chars().any(|c| c.is_whitespace() || c.is_control())
        && !email.contains('\0')
}
fn random() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| "GMAIL_OAUTH_RANDOM")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
#[cfg(test)]
fn grant(
    data: &Value,
    email: &str,
    previous_refresh: &str,
    require_scope: bool,
) -> Result<Grant, String> {
    grant_for(data, email, previous_refresh, require_scope, MAIL_SCOPE)
}
fn grant_for(
    data: &Value,
    email: &str,
    previous_refresh: &str,
    require_scope: bool,
    scope: &str,
) -> Result<Grant, String> {
    let access = data["access_token"].as_str().unwrap_or("");
    let refresh = data["refresh_token"].as_str().unwrap_or(previous_refresh);
    let expires = data["expires_in"]
        .as_i64()
        .filter(|n| *n > 0 && *n <= 86400)
        .ok_or("GMAIL_OAUTH_RESPONSE")?;
    if access.is_empty()
        || refresh.is_empty()
        || access.len() > 16384
        || refresh.len() > 16384
        || [access, refresh]
            .iter()
            .any(|s| s.chars().any(char::is_whitespace) || s.contains('\0'))
        || !data["token_type"]
            .as_str()
            .is_some_and(|s| s.eq_ignore_ascii_case("Bearer"))
    {
        return Err("GMAIL_OAUTH_RESPONSE".into());
    }
    if (require_scope || data.get("scope").is_some())
        && !data["scope"]
            .as_str()
            .unwrap_or("")
            .split_whitespace()
            .any(|s| {
                s == scope
                    || (scope == MICROSOFT_SCOPE
                        && s == "https://outlook.office365.com/IMAP.AccessAsUser.All")
            })
    {
        return Err("GMAIL_OAUTH_SCOPE".into());
    }
    Ok(Grant {
        email: email.into(),
        access_token: access.into(),
        refresh_token: refresh.into(),
        expires_at: chrono::Utc::now().timestamp() + expires,
        invalid: false,
        persistent: false,
    })
}

impl Manager {
    pub fn microsoft() -> Self {
        Self {
            provider: Provider::Microsoft,
            store: Arc::new(Keychain("com.imapsync.studio.microsoft-oauth")),
            api: Arc::new(Microsoft),
            ..Self::default()
        }
    }
    pub fn load(path: PathBuf) -> Self {
        Self::load_for(path, Provider::Google)
    }
    pub fn load_microsoft(path: PathBuf) -> Self {
        Self::load_for(path, Provider::Microsoft)
    }
    fn load_for(path: PathBuf, provider: Provider) -> Self {
        let mut manager = if provider == Provider::Microsoft {
            Self::microsoft()
        } else {
            Self::default()
        };
        manager.path = Some(path.clone());
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                let mut state = manager.state.lock().unwrap();
                state.config.client_id = value["clientId"].as_str().unwrap_or("").into();
                state.config.remember = value["remember"].as_bool().unwrap_or(false);
                if state.config.remember {
                    state.config.client_secret = manager
                        .store
                        .get(&key(&state.config.client_id, "client-secret"))
                        .unwrap_or_default();
                    state.config.secret_persistent = !state.config.client_secret.is_empty();
                    for email in value["accounts"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                    {
                        if let Ok(encoded) = manager.store.get(&key(&state.config.client_id, email))
                        {
                            if let Ok(mut account) = serde_json::from_str::<Grant>(&encoded) {
                                if account.email == email {
                                    account.persistent = true;
                                    state.accounts.insert(email.into(), account);
                                }
                            }
                        }
                    }
                }
            }
        }
        manager
    }
    fn metadata(&self, state: &State) -> Result<(), String> {
        if let Some(path) = &self.path {
            let parent = path.parent().ok_or("GMAIL_OAUTH_STORAGE")?;
            std::fs::create_dir_all(parent).map_err(|_| "GMAIL_OAUTH_STORAGE")?;
            let data = json!({"clientId":state.config.client_id,"remember":state.config.remember,
                "accounts":state.accounts.values().filter(|g|g.persistent).map(|g| &g.email).collect::<Vec<_>>()});
            write_private(
                path,
                &serde_json::to_vec(&data).map_err(|_| "GMAIL_OAUTH_STORAGE")?,
            )?;
        }
        Ok(())
    }
    pub fn view(&self) -> Value {
        let state = self.state.lock().unwrap();
        json!({"clientId":state.config.client_id,"remember":state.config.remember,
            "configured":!state.config.client_id.is_empty(),
            "accounts":state.accounts.values().map(|g|json!({"email":g.email,"connected":!g.invalid,"persistent":g.persistent})).collect::<Vec<_>>(),
            "attempt":state.attempt.as_ref().map(|a|json!({"id":a.id,"status":a.status,"email":a.email,"error":a.error}))})
    }
    pub fn configure(&self, body: &Value) -> Result<Value, String> {
        let supplied = body["clientId"].as_str().unwrap_or("").trim();
        let normalized = if self.provider == Provider::Microsoft {
            supplied.to_lowercase()
        } else {
            supplied.to_string()
        };
        let id = normalized.as_str();
        if (self.provider == Provider::Google
            && (!id.ends_with(".apps.googleusercontent.com")
                || id.len() > 256
                || !id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))))
            || (self.provider == Provider::Microsoft
                && (!valid_guid(id)
                    || body["clientSecret"].as_str().is_some_and(|s| !s.is_empty())))
        {
            return Err("GMAIL_OAUTH_CLIENT".into());
        }
        let mut state = self.state.lock().unwrap();
        if state
            .attempt
            .as_ref()
            .is_some_and(|a| a.status == "waiting")
        {
            return Err("GMAIL_OAUTH_BUSY".into());
        }
        let secret = body["clientSecret"].as_str().unwrap_or("");
        if secret.len() > 4096 || secret.contains(['\0', '\r', '\n']) {
            return Err("GMAIL_OAUTH_CLIENT".into());
        }
        if id != state.config.client_id {
            for account in state.accounts.values().filter(|a| a.persistent) {
                self.store
                    .delete(&key(&state.config.client_id, &account.email))?;
            }
            if state.config.secret_persistent {
                self.store
                    .delete(&key(&state.config.client_id, "client-secret"))?;
            }
            state.config.secret_persistent = false;
            state.accounts.clear();
            state.config.client_secret.clear();
        }
        state.config.client_id = id.into();
        if !secret.is_empty() {
            state.config.client_secret = secret.into();
        }
        state.config.remember = body["remember"] == true;
        let mut saved = if state.config.remember {
            self.store
                .put(&key(id, "client-secret"), &state.config.client_secret)
                .is_ok()
        } else {
            if state.config.secret_persistent {
                self.store.delete(&key(id, "client-secret"))?;
            }
            true
        };
        state.config.secret_persistent = state.config.remember && saved;
        if state.config.remember {
            for account in state.accounts.values_mut() {
                account.persistent = self
                    .store
                    .put(
                        &key(id, &account.email),
                        &serde_json::to_string(account).map_err(|_| "GMAIL_OAUTH_STORAGE")?,
                    )
                    .is_ok();
                saved &= account.persistent;
            }
        }
        // Disabling persistence also removes old saved grants, never just hides them.
        if !state.config.remember {
            for account in state.accounts.values_mut() {
                if account.persistent {
                    self.store.delete(&key(id, &account.email))?;
                }
                account.persistent = false;
            }
        }
        self.metadata(&state)?;
        drop(state);
        let mut view = self.view();
        view["storageAvailable"] = json!(saved);
        Ok(view)
    }
    fn save_grant(&self, state: &mut State, mut account: Grant) -> Result<(), String> {
        if state.config.remember {
            account.persistent = self
                .store
                .put(
                    &key(&state.config.client_id, &account.email),
                    &serde_json::to_string(&account).map_err(|_| "GMAIL_OAUTH_STORAGE")?,
                )
                .is_ok();
        }
        state.accounts.insert(account.email.clone(), account);
        self.metadata(state)
    }
    #[cfg(test)]
    pub fn with_test_accounts(emails: &[&str]) -> Self {
        let manager = Self::default();
        for email in emails {
            manager.state.lock().unwrap().accounts.insert(
                (*email).into(),
                Grant {
                    email: (*email).into(),
                    access_token: "test-access-token".into(),
                    refresh_token: "test-refresh-token".into(),
                    expires_at: chrono::Utc::now().timestamp() + 3600,
                    invalid: false,
                    persistent: false,
                },
            );
        }
        manager
    }
    #[cfg(test)]
    pub fn with_microsoft_test_accounts(emails: &[&str]) -> Self {
        let mut manager = Self::with_test_accounts(emails);
        manager.provider = Provider::Microsoft;
        manager.api = Arc::new(Microsoft);
        for account in manager.state.lock().unwrap().accounts.values_mut() {
            account.access_token = "test-microsoft-access-token".into();
        }
        manager
    }
    pub fn ready(&self, email: &str) -> bool {
        self.state
            .lock()
            .unwrap()
            .accounts
            .get(&email.trim().to_lowercase())
            .is_some_and(|a| !a.invalid)
    }
    pub fn token(&self, email: &str) -> Result<String, String> {
        let email = email.trim().to_lowercase();
        let mut state = self.state.lock().unwrap();
        let old = state
            .accounts
            .get(&email)
            .cloned()
            .filter(|a| !a.invalid)
            .ok_or("GMAIL_OAUTH_RECONNECT")?;
        if old.expires_at > chrono::Utc::now().timestamp() + 90 {
            return Ok(old.access_token);
        }
        let data = match self.api.refresh(&state.config, &old.refresh_token) {
            Ok(data) => data,
            Err(error) => {
                if error == "GMAIL_OAUTH_RECONNECT" {
                    state.accounts.get_mut(&email).unwrap().invalid = true;
                }
                return Err(error);
            }
        };
        let refreshed = match grant_for(
            &data,
            &email,
            &old.refresh_token,
            false,
            self.provider.scope(),
        ) {
            Ok(grant) => grant,
            Err(error) => {
                state.accounts.get_mut(&email).unwrap().invalid = true;
                return Err(error);
            }
        };
        let token = refreshed.access_token.clone();
        self.save_grant(&mut state, refreshed)?;
        Ok(token)
    }
    pub fn forget(&self, email: &str) -> Result<(), String> {
        let email = email.trim().to_lowercase();
        let mut state = self.state.lock().unwrap();
        if state.accounts.get(&email).is_some_and(|a| a.persistent) {
            self.store.delete(&key(&state.config.client_id, &email))?;
        }
        state.accounts.remove(&email);
        self.metadata(&state)
    }
    pub fn cancel(&self) {
        if let Some(attempt) = &self.state.lock().unwrap().attempt {
            attempt.cancel.store(true, Ordering::Release);
        }
    }
    pub fn begin(self: &Arc<Self>, email: &str, lang: &str) -> Result<Value, String> {
        let expected = email.trim().to_lowercase();
        if !expected.is_empty() && !valid_email(&expected) {
            return Err("GMAIL_OAUTH_IDENTITY".into());
        }
        let config = {
            let state = self.state.lock().unwrap();
            if state.config.client_id.is_empty() {
                return Err("GMAIL_OAUTH_CLIENT".into());
            }
            if state
                .attempt
                .as_ref()
                .is_some_and(|a| a.status == "waiting")
            {
                return Err("GMAIL_OAUTH_BUSY".into());
            }
            state.config.clone()
        };
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| "GMAIL_OAUTH_CALLBACK")?;
        listener
            .set_nonblocking(true)
            .map_err(|_| "GMAIL_OAUTH_CALLBACK")?;
        let redirect = format!(
            "http://{}:{}/oauth/callback",
            if self.provider == Provider::Microsoft {
                "localhost"
            } else {
                "127.0.0.1"
            },
            listener
                .local_addr()
                .map_err(|_| "GMAIL_OAUTH_CALLBACK")?
                .port()
        );
        let nonce = random()?;
        let verifier = random()?;
        let id = random()?;
        let oidc_nonce = random()?;
        let mut url = authorization_url(
            self.provider,
            &config.client_id,
            &redirect,
            &nonce,
            &verifier,
            &oidc_nonce,
        );
        if !expected.is_empty() {
            url.query_pairs_mut().append_pair("login_hint", &expected);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let mut current = self.state.lock().unwrap();
        if current
            .attempt
            .as_ref()
            .is_some_and(|a| a.status == "waiting")
            || current.config.client_id != config.client_id
        {
            return Err("GMAIL_OAUTH_BUSY".into());
        }
        current.attempt = Some(Attempt {
            id: id.clone(),
            status: "waiting".into(),
            email: expected.clone(),
            error: String::new(),
            cancel: cancel.clone(),
        });
        drop(current);
        if open_browser(url.as_str()).is_err() {
            self.finish(&id, Err("GMAIL_OAUTH_BROWSER".into()));
            return Err("GMAIL_OAUTH_BROWSER".into());
        }
        let manager = self.clone();
        let language = lang.to_string();
        let worker_id = id.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let code = wait_callback(
                    listener,
                    &redirect,
                    &nonce,
                    &cancel,
                    &language,
                    Duration::from_secs(180),
                )?;
                if cancel.load(Ordering::Acquire) {
                    return Err("GMAIL_OAUTH_CANCELLED".into());
                }
                let data = manager.api.exchange(&config, &code, &redirect, &verifier)?;
                let email = manager.api.identity_response(&data, &config, &oidc_nonce)?;
                if !expected.is_empty() && expected != email {
                    return Err("GMAIL_OAUTH_IDENTITY_MISMATCH".into());
                }
                let account = grant_for(&data, &email, "", true, manager.provider.scope())?;
                let mut state = manager.state.lock().unwrap();
                if cancel.load(Ordering::Acquire) {
                    return Err("GMAIL_OAUTH_CANCELLED".into());
                }
                manager.save_grant(&mut state, account)?;
                Ok(email)
            })();
            manager.finish(&worker_id, result);
        });
        Ok(json!({"id":id}))
    }
    fn finish(&self, id: &str, result: Result<String, String>) {
        let result = result.map_err(|e| self.provider.error(e));
        let mut state = self.state.lock().unwrap();
        if let Some(a) = state.attempt.as_mut().filter(|a| a.id == id) {
            match result {
                Ok(email) => {
                    a.status = "connected".into();
                    a.email = email;
                }
                Err(error) => {
                    a.status = if error.ends_with("OAUTH_CANCELLED") {
                        "cancelled"
                    } else {
                        "error"
                    }
                    .into();
                    a.error = error;
                }
            }
        }
    }
}

fn open_browser(url: &str) -> Result<(), String> {
    // Only URLs constructed from the fixed Google authorization endpoint reach this helper.
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler");
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = Command::new("xdg-open");
    let mut child = command
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "GMAIL_OAUTH_BROWSER")?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn callback_code(target: &str, redirect: &str, nonce: &str) -> Result<Option<String>, String> {
    let url =
        Url::parse(&format!("http://127.0.0.1{target}")).map_err(|_| "GMAIL_OAUTH_CALLBACK")?;
    let expected = Url::parse(redirect).map_err(|_| "GMAIL_OAUTH_CALLBACK")?;
    if url.path() != expected.path() {
        return Ok(None);
    }
    let mut pairs = HashMap::new();
    for (name, value) in url.query_pairs() {
        if pairs.insert(name.to_string(), value.to_string()).is_some() {
            return Ok(None);
        }
    }
    let state = pairs.get("state").map(String::as_str).unwrap_or("");
    if state.len() != nonce.len()
        || state
            .bytes()
            .zip(nonce.bytes())
            .fold(0u8, |v, (a, b)| v | (a ^ b))
            != 0
    {
        return Ok(None);
    }
    if pairs.contains_key("error") {
        return Err("GMAIL_OAUTH_DENIED".into());
    }
    Ok(pairs
        .remove("code")
        .filter(|s| !s.is_empty() && s.len() <= 4096))
}
fn wait_callback(
    listener: TcpListener,
    redirect: &str,
    nonce: &str,
    cancel: &AtomicBool,
    lang: &str,
    timeout: Duration,
) -> Result<String, String> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if cancel.load(Ordering::Acquire) {
            return Err("GMAIL_OAUTH_CANCELLED".into());
        }
        match listener.accept() {
            Ok((mut stream, peer)) => {
                if !peer.ip().is_loopback() {
                    continue;
                }
                stream
                    .set_nonblocking(false)
                    .map_err(|_| "GMAIL_OAUTH_CALLBACK")?;
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .map_err(|_| "GMAIL_OAUTH_CALLBACK")?;
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .map_err(|_| "GMAIL_OAUTH_CALLBACK")?;
                let mut request = String::new();
                let mut reader = BufReader::new((&mut stream).take(16385));
                if reader.read_line(&mut request).is_err() || request.len() > 8192 {
                    continue;
                }
                let mut total = request.len();
                let mut complete = false;
                loop {
                    let mut header = String::new();
                    let count = match reader.read_line(&mut header) {
                        Ok(n) => n,
                        Err(_) => break,
                    };
                    total += count;
                    if count == 0 || total > 16384 {
                        break;
                    }
                    if header == "\r\n" || header == "\n" {
                        complete = true;
                        break;
                    }
                }
                drop(reader);
                if !complete {
                    continue;
                }
                let parts = request.split_whitespace().collect::<Vec<_>>();
                let result = if parts.len() == 3 && parts[0] == "GET" {
                    callback_code(parts[1], redirect, nonce)
                } else {
                    Ok(None)
                };
                let accepted = !matches!(result, Ok(None));
                let message=match lang {"ja"=>"認証応答を受け取りました。このタブを閉じ、Imapsync Studioに戻ってください。","zh"=>"已收到授权响应。请关闭此标签并返回 Imapsync Studio。",_=>"Authorization response received. Close this tab and return to Imapsync Studio."};
                let body = if accepted {
                    message
                } else {
                    "Invalid authorization response."
                };
                let status = if accepted {
                    "200 OK"
                } else {
                    "400 Bad Request"
                };
                let response=format!("HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",body.len());
                let _ = stream.write_all(response.as_bytes());
                match result {
                    Ok(Some(code)) => return Ok(code),
                    Err(error) => return Err(error),
                    Ok(None) => {}
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(40))
            }
            Err(_) => return Err("GMAIL_OAUTH_CALLBACK".into()),
        }
    }
    Err("GMAIL_OAUTH_TIMEOUT".into())
}

pub fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().ok_or("GMAIL_OAUTH_STORAGE")?)
        .map_err(|_| "GMAIL_OAUTH_STORAGE")?;
    file.write_all(bytes).map_err(|_| "GMAIL_OAUTH_STORAGE")?;
    file.persist(path).map_err(|_| "GMAIL_OAUTH_STORAGE")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Memory(Mutex<HashMap<String, String>>);
    impl SecretStore for Memory {
        fn get(&self, k: &str) -> Result<String, String> {
            self.0
                .lock()
                .unwrap()
                .get(k)
                .cloned()
                .ok_or("missing".into())
        }
        fn put(&self, k: &str, v: &str) -> Result<(), String> {
            self.0.lock().unwrap().insert(k.into(), v.into());
            Ok(())
        }
        fn delete(&self, k: &str) -> Result<(), String> {
            self.0.lock().unwrap().remove(k);
            Ok(())
        }
    }
    struct MockGoogle(bool);
    impl OAuthApi for MockGoogle {
        fn exchange(&self, _: &Config, _: &str, _: &str, _: &str) -> Result<Value, String> {
            Err("unused".into())
        }
        fn identity(&self, _: &str) -> Result<String, String> {
            Ok("a@gmail.com".into())
        }
        fn refresh(&self, _: &Config, token: &str) -> Result<Value, String> {
            assert_eq!(token, "refresh");
            if self.0 {
                Err("GMAIL_OAUTH_RECONNECT".into())
            } else {
                Ok(
                    json!({"access_token":"renewed","refresh_token":"rotated","expires_in":3600,"token_type":"Bearer"}),
                )
            }
        }
    }
    fn manager(invalid: bool) -> Manager {
        Manager {
            provider: Provider::Google,
            state: Mutex::new(State::default()),
            path: None,
            store: Arc::new(Memory::default()),
            api: Arc::new(MockGoogle(invalid)),
        }
    }
    fn data() -> Value {
        json!({"access_token":"access","refresh_token":"refresh","expires_in":3600,"token_type":"Bearer","scope":MAIL_SCOPE})
    }
    struct Unavailable;
    impl SecretStore for Unavailable {
        fn get(&self, _: &str) -> Result<String, String> {
            Err("GMAIL_OAUTH_STORAGE".into())
        }
        fn put(&self, _: &str, _: &str) -> Result<(), String> {
            Err("GMAIL_OAUTH_STORAGE".into())
        }
        fn delete(&self, _: &str) -> Result<(), String> {
            Err("GMAIL_OAUTH_STORAGE".into())
        }
    }
    #[test]
    fn unavailable_keychain_allows_session_auth_and_disconnect() {
        let mut m = manager(false);
        m.store = Arc::new(Unavailable);
        let view = m
            .configure(&json!({"clientId":"test.apps.googleusercontent.com","remember":true}))
            .unwrap();
        assert_eq!(view["storageAvailable"], false);
        m.save_grant(
            &mut m.state.lock().unwrap(),
            grant(&data(), "a@gmail.com", "", true).unwrap(),
        )
        .unwrap();
        assert!(m.ready("a@gmail.com"));
        assert_eq!(m.view()["accounts"][0]["persistent"], false);
        m.forget("a@gmail.com").unwrap();
        assert!(!m.ready("a@gmail.com"));
        m.configure(&json!({"clientId":"new.apps.googleusercontent.com","remember":false}))
            .unwrap();
    }
    #[test]
    fn loopback_callback_ignores_spoofed_state_then_accepts_code() {
        use std::net::TcpStream;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let redirect = format!("http://{addr}/oauth/callback");
        let worker = std::thread::spawn(move || {
            wait_callback(
                listener,
                &redirect,
                "expected",
                &AtomicBool::new(false),
                "ja",
                Duration::from_secs(5),
            )
        });
        for state in ["spoofed", "expected"] {
            let mut stream = TcpStream::connect(addr).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            write!(
                stream,
                "GET /oauth/callback?state={state}&code=opaque-auth-code HTTP/1.1\r\nHost: {addr}\r\n\r\n"
            )
            .unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            assert!(response.contains("Cache-Control: no-store"));
            assert!(!response.contains("opaque-auth-code"));
        }
        assert_eq!(worker.join().unwrap().unwrap(), "opaque-auth-code");
    }
    #[test]
    fn pkce_matches_rfc7636() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        assert_eq!(random().unwrap().len(), 43);
    }
    #[test]
    fn callback_rejects_state_duplicates_and_wrong_path() {
        let uri = "http://127.0.0.1:123/oauth/callback";
        assert_eq!(
            callback_code("/oauth/callback?code=abc&state=good", uri, "good").unwrap(),
            Some("abc".into())
        );
        for target in [
            "/oauth/callback?code=a&state=bad",
            "/oauth/callback?code=a&state=good&state=good",
            "/wrong?code=a&state=good",
        ] {
            assert_eq!(callback_code(target, uri, "good").unwrap(), None);
        }
        assert_eq!(
            callback_code(
                "/oauth/callback?error=access_denied&state=good",
                uri,
                "good"
            )
            .unwrap_err(),
            "GMAIL_OAUTH_DENIED"
        );
    }
    #[test]
    fn requires_mail_scope_and_refresh_token() {
        let mut v = data();
        v["scope"] = json!("openid email");
        assert!(grant(&v, "a@gmail.com", "", true).is_err());
        v = data();
        v["refresh_token"] = Value::Null;
        assert!(grant(&v, "a@gmail.com", "", true).is_err());
        v = data();
        v["access_token"] = json!("bad\ntoken");
        assert!(grant(&v, "a@gmail.com", "", true).is_err());
    }
    #[test]
    fn refresh_rotates_and_view_never_exposes_secrets() {
        let m = manager(false);
        m.configure(&json!({"clientId":"test.apps.googleusercontent.com","remember":true,"clientSecret":"client-secret"})).unwrap();
        let mut account = grant(&data(), "a@gmail.com", "", true).unwrap();
        account.expires_at = 0;
        m.save_grant(&mut m.state.lock().unwrap(), account).unwrap();
        assert_eq!(m.token("A@gmail.com").unwrap(), "renewed");
        let state = m.state.lock().unwrap();
        assert_eq!(state.accounts["a@gmail.com"].refresh_token, "rotated");
        drop(state);
        let view = m.view().to_string();
        for secret in [
            "renewed",
            "rotated",
            "client-secret",
            "refresh_token",
            "access_token",
        ] {
            assert!(!view.contains(secret));
        }
        m.forget("a@gmail.com").unwrap();
        assert!(!m.ready("a@gmail.com"));
    }
    #[test]
    fn revoked_refresh_requires_reconnect() {
        let m = manager(true);
        let mut account = grant(&data(), "a@gmail.com", "", true).unwrap();
        account.expires_at = 0;
        m.save_grant(&mut m.state.lock().unwrap(), account).unwrap();
        assert_eq!(m.token("a@gmail.com").unwrap_err(), "GMAIL_OAUTH_RECONNECT");
        assert!(!m.ready("a@gmail.com"));
    }
    #[test]
    fn metadata_contains_no_tokens_and_disabling_remember_erases_store() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = manager(false);
        m.path = Some(dir.path().join("oauth.json"));
        let config = json!({"clientId":"test.apps.googleusercontent.com","remember":true,"clientSecret":"secret-client"});
        m.configure(&config).unwrap();
        m.save_grant(
            &mut m.state.lock().unwrap(),
            grant(&data(), "a@gmail.com", "", true).unwrap(),
        )
        .unwrap();
        let raw = std::fs::read_to_string(m.path.as_ref().unwrap()).unwrap();
        assert!(!raw.contains("refresh"));
        assert!(!raw.contains("access"));
        assert!(!raw.contains("secret-client"));
        m.configure(&json!({"clientId":"test.apps.googleusercontent.com","remember":false}))
            .unwrap();
        assert!(m
            .store
            .get(&key("test.apps.googleusercontent.com", "a@gmail.com"))
            .is_err());
        assert!(m.ready("a@gmail.com"));
    }
    #[test]
    fn callback_cancellation_and_timeout() {
        let cancel = AtomicBool::new(true);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        assert_eq!(
            wait_callback(
                listener,
                "http://127.0.0.1/oauth/callback",
                "state",
                &cancel,
                "en",
                Duration::from_secs(1)
            )
            .unwrap_err(),
            "GMAIL_OAUTH_CANCELLED"
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        assert_eq!(
            wait_callback(
                listener,
                "http://127.0.0.1/oauth/callback",
                "state",
                &AtomicBool::new(false),
                "en",
                Duration::ZERO
            )
            .unwrap_err(),
            "GMAIL_OAUTH_TIMEOUT"
        );
    }
    #[test]
    fn microsoft_pkce_request_and_signed_identity_are_bound_to_client_nonce_and_tenant() {
        use rsa::{
            pkcs1v15::SigningKey,
            signature::{SignatureEncoding, Signer},
            traits::PublicKeyParts,
            RsaPrivateKey,
        };
        // Generate synthetic credentials in memory for this test run. Never store
        // a complete provider-shaped token or private signing key in the repo.
        let private = RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048).unwrap();
        let public = private.to_public_key();
        let client = "11111111-1111-1111-1111-111111111111";
        let nonce = "test-nonce";
        let now = 1800000000;
        let tenant = "22222222-2222-2222-2222-222222222222";
        let claims = json!({"aud":client,"tid":tenant,"iss":format!("https://login.microsoftonline.com/{tenant}/v2.0"),"nonce":nonce,"iat":now,"exp":now+3600,"nbf":now-10,"preferred_username":"USER@Example.test","sub":"synthetic-subject"});
        let header = json!({"alg":"RS256","kid":"generated-test-key","typ":"JWT"});
        let message = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap()),
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())
        );
        let signature = SigningKey::<Sha256>::new(private).sign(message.as_bytes());
        let encoded = format!("{message}.{}", URL_SAFE_NO_PAD.encode(signature.to_vec()));
        let token = encoded.as_str();
        let key_set = json!({"keys":[{"kty":"RSA","use":"sig","alg":"RS256","kid":"generated-test-key","n":URL_SAFE_NO_PAD.encode(public.n().to_bytes_be()),"e":URL_SAFE_NO_PAD.encode(public.e().to_bytes_be()),"issuer":"https://login.microsoftonline.com/{tenantid}/v2.0"}]});
        let keys = &key_set;
        let url = authorization_url(
            Provider::Microsoft,
            client,
            "http://localhost:12345/oauth/callback",
            "state",
            "verifier",
            nonce,
        );
        assert_eq!(url.host_str(), Some("login.microsoftonline.com"));
        let query = url.query_pairs().into_owned().collect::<HashMap<_, _>>();
        assert_eq!(query["scope"], MICROSOFT_AUTH_SCOPE);
        assert_eq!(query["code_challenge"], challenge("verifier"));
        assert_eq!(query["nonce"], nonce);
        assert!(!query.contains_key("client_secret"));
        assert_eq!(
            microsoft_identity(token, client, nonce, keys, now).unwrap(),
            "user@example.test"
        );
        assert!(microsoft_identity(token, client, "different-nonce", keys, now).is_err());
        assert!(microsoft_identity(
            token,
            "33333333-3333-3333-3333-333333333333",
            nonce,
            keys,
            now
        )
        .is_err());
        assert!(microsoft_identity(token, client, nonce, keys, now + 4000).is_err());
        assert!(microsoft_identity(token, client, nonce, keys, now - 3600).is_err());
        let mut changed = keys.clone();
        changed["keys"][0]["issuer"] = json!("https://attacker.invalid/{tenantid}/v2.0");
        assert!(microsoft_identity(token, client, nonce, &changed, now).is_err());
        changed = keys.clone();
        changed["keys"][0]["kid"] = json!("different");
        assert!(microsoft_identity(token, client, nonce, &changed, now).is_err());
        changed = keys.clone();
        changed["keys"][0]["n"] = json!("AQAB");
        assert!(microsoft_identity(token, client, nonce, &changed, now).is_err());
        let mut parts = token.split('.').map(str::to_string).collect::<Vec<_>>();
        let mut claims: Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(&parts[1]).unwrap()).unwrap();
        claims["preferred_username"] = json!("attacker@example.test");
        parts[1] = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
        assert!(microsoft_identity(&parts.join("."), client, nonce, keys, now).is_err());
    }
    #[test]
    fn microsoft_configuration_scope_refresh_and_metadata_stay_separate() {
        let mut ms = Manager::microsoft();
        ms.store = Arc::new(Memory::default());
        ms.api = Arc::new(MockGoogle(false));
        let dir = tempfile::tempdir().unwrap();
        ms.path = Some(dir.path().join("microsoft.json"));
        let client = "11111111-1111-1111-1111-111111111111";
        assert!(ms
            .configure(&json!({"clientId":"google.apps.googleusercontent.com"}))
            .is_err());
        assert!(ms
            .configure(&json!({"clientId":client,"clientSecret":"not-a-public-client"}))
            .is_err());
        ms.configure(&json!({"clientId":client,"remember":true}))
            .unwrap();
        let mut data = data();
        data["scope"] = json!(MICROSOFT_SCOPE);
        assert!(grant_for(&data, "user@example.test", "", true, MAIL_SCOPE).is_err());
        let mut account = grant_for(&data, "user@example.test", "", true, MICROSOFT_SCOPE).unwrap();
        account.expires_at = 0;
        ms.save_grant(&mut ms.state.lock().unwrap(), account)
            .unwrap();
        assert_eq!(ms.token("USER@EXAMPLE.TEST").unwrap(), "renewed");
        assert_eq!(
            ms.state.lock().unwrap().accounts["user@example.test"].refresh_token,
            "rotated"
        );
        assert!(!Manager::default().ready("user@example.test"));
        let text = std::fs::read_to_string(ms.path.as_ref().unwrap()).unwrap();
        let view = ms.view().to_string();
        for text in [text, view] {
            for secret in ["access_token", "refresh_token", "renewed", "rotated"] {
                assert!(!text.contains(secret));
            }
        }
        data["scope"] = json!("https://graph.microsoft.com/Mail.Read");
        assert!(grant_for(&data, "user@example.test", "", true, MICROSOFT_SCOPE).is_err());
        ms.forget("user@example.test").unwrap();
        assert!(!ms.ready("user@example.test"));
    }
    #[test]
    fn microsoft_loopback_callback_accepts_registered_localhost_path() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let redirect = format!("http://localhost:{port}/oauth/callback");
        let worker = std::thread::spawn(move || {
            wait_callback(
                listener,
                &redirect,
                "state-nonce",
                &AtomicBool::new(false),
                "en",
                Duration::from_secs(3),
            )
        });
        let mut stream = std::net::TcpStream::connect(("localhost", port)).unwrap();
        stream.write_all(format!("GET /oauth/callback?state=state-nonce&code=code-value HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n").as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.contains("200 OK"));
        assert_eq!(worker.join().unwrap().unwrap(), "code-value");
    }
}
