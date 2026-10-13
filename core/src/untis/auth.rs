use crate::env::{Env, HttpResponse};
use crate::errors::ApiError;
use crate::settings::Cookies;
use crate::store::Store;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::json;
use std::collections::HashMap;
use std::marker::PhantomData;
use totp_rs::{Algorithm, Secret, TOTP};

/// How long before its expiry a cached token is replaced, so it can't run out mid-request
const TOKEN_EXPIRY_MARGIN_MS: u64 = 60_000;

pub struct AuthHelper<E: Env>(PhantomData<E>);

impl<E: Env> AuthHelper<E> {
    pub fn is_authenticated() -> bool {
        Store::<E>::get_cookies().is_some()
    }

    pub async fn authenticate(
        school_name: String,
        username: String,
        secret: String,
    ) -> Result<(), ApiError> {
        if school_name.is_empty() || username.is_empty() || secret.is_empty() {
            return Err(ApiError::Authentication("Credentials not set".to_string()));
        }

        let env = E::default();

        let secret_bytes = Secret::Encoded(secret)
            .to_bytes()
            .map_err(|x| ApiError::Authentication(x.to_string()))?;

        let now_ms = env.now_ms();
        let totp = TOTP::new_unchecked(Algorithm::SHA1, 6, 1, 30, secret_bytes);
        let token = totp.generate(now_ms / 1000);

        let login_url = format!(
            "https://{}.webuntis.com/WebUntis/jsonrpc_intern.do?m=getUserData2017&school={}&v=i2.2",
            school_name, school_name
        );

        let body = json!({
            "id": "opensource-client",
            "method": "getUserData2017",
            "params": [{"auth": {"clientTime": now_ms, "user": username, "otp": token}}],
            "jsonrpc": "2.0"
        });

        let response = env
            .request("POST", &login_url, HashMap::new(), body.to_string())
            .await
            .map_err(ApiError::Network)?;

        let response_json: serde_json::Value =
            serde_json::from_str(&response.body).map_err(|x| ApiError::Parsing(x.to_string()))?;

        if let Some(error) = response_json.get("error") {
            let message = error["message"].as_str().unwrap_or("Unknown error");
            let code = error["code"].as_i64().unwrap_or(0);
            return Err(ApiError::Authentication(format!(
                "API Error {}: {}",
                code, message
            )));
        }

        let mut jsessionid = None;
        let mut tenant_id = None;
        let mut school_name_cookie = None;

        for val in response.header("set-cookie") {
            let first_part = val.split(';').next().unwrap_or("");
            let mut parts = first_part.splitn(2, '=');
            let key = parts.next().unwrap_or("").trim();
            let value = parts.next().unwrap_or("").trim().to_string();

            match key {
                "JSESSIONID" => jsessionid = Some(value),
                "Tenant-Id" => tenant_id = Some(value),
                "schoolname" => school_name_cookie = Some(value),
                _ => {}
            }
        }

        if let (Some(jsessionid), Some(tenant_id), Some(school_name_cookie)) =
            (jsessionid, tenant_id, school_name_cookie)
        {
            let cookies = Cookies {
                jsessionid,
                tenant_id,
                school_name_base32: school_name_cookie,
            };
            Store::<E>::save_cookies(&cookies).map_err(ApiError::Miscellaneous)?;
        }

        Ok(())
    }

    /// Authenticates with the credentials from the settings, returning the fresh cookies
    async fn reauthenticate() -> Result<Cookies, ApiError> {
        let settings = Store::<E>::get_settings()?
            .ok_or(ApiError::Miscellaneous("Settings are empty".to_string()))?;

        Self::authenticate(
            settings.untis_auth.school_identifier,
            settings.untis_auth.user_identifier,
            settings.untis_auth.secret,
        )
        .await?;

        Store::<E>::get_cookies().ok_or(ApiError::Authentication(
            "Could not get cookies after authenticating".to_string(),
        ))
    }

    /// A JWT is only accepted if it really is one: Untis answers an expired session with a JSON
    /// error body rather than a status code, so anything that isn't three dot-separated parts
    /// means the session is gone and we have to log in again.
    fn as_jwt(body: &str) -> Option<String> {
        let body = body.trim();
        (body.split('.').count() == 3 && !body.starts_with('{')).then(|| body.to_string())
    }

    async fn request_token(cookies: &Cookies, school_name: &str) -> Result<String, ApiError> {
        let url = format!("https://{}.webuntis.com/WebUntis/api/token/new", school_name);
        let mut headers = HashMap::new();
        headers.insert("Cookie".to_string(), vec![cookies.to_header_value()]);

        let response = E::default().request("GET", &url, headers, String::new()).await?;
        Ok(response.body)
    }

    async fn get_token() -> Result<String, ApiError> {
        let settings = Store::<E>::get_settings()?
            .ok_or(ApiError::Miscellaneous("Settings are empty".to_string()))?;

        if !settings.untis_auth.is_complete() {
            return Err(ApiError::Authentication("Credentials not set".to_string()));
        }
        let school_name = &settings.untis_auth.school_identifier;

        // a token is good for many requests, fetching one per request doubles every round trip
        let now_ms = E::default().now_ms();
        if let Some(token) = Store::<E>::get_token()
            && token_expiry_ms(&token).is_some_and(|expiry| expiry > now_ms + TOKEN_EXPIRY_MARGIN_MS) {
            return Ok(token);
        }

        let cookies = match Store::<E>::get_cookies() {
            Some(cookies) => cookies,
            None => Self::reauthenticate().await?,
        };

        let body = Self::request_token(&cookies, school_name).await?;
        if let Some(token) = Self::as_jwt(&body) {
            return Self::cache_token(token);
        }

        let fresh_cookies = Self::reauthenticate().await?;
        let retry_body = Self::request_token(&fresh_cookies, school_name).await?;
        let token = Self::as_jwt(&retry_body).ok_or_else(|| {
            ApiError::Authentication(format!("Failed to obtain JWT token: {}", retry_body.trim()))
        })?;
        Self::cache_token(token)
    }

    fn cache_token(token: String) -> Result<String, ApiError> {
        Store::<E>::save_token(&token).map_err(ApiError::Miscellaneous)?;
        Ok(token)
    }

    /// Performs a request authenticated with the session cookies. The classreg API is older than
    /// the token one and only knows the session, so a bearer token is no use to it.
    pub async fn session_request(
        method: &str,
        url: &str,
        headers: HashMap<String, Vec<String>>,
        body: String,
    ) -> Result<HttpResponse, ApiError> {
        let cookies = match Store::<E>::get_cookies() {
            Some(cookies) => cookies,
            None => Self::reauthenticate().await?,
        };
        let res = Self::send_with_session(&cookies, method, url, headers.clone(), body.clone()).await?;

        // a session that's gone gets the login page rather than an error, so anything that isn't
        // JSON means we have to log in again instead of that the request itself failed
        if res.body.trim_start().starts_with('{') {
            return Ok(res);
        }

        let fresh_cookies = Self::reauthenticate().await?;
        Self::send_with_session(&fresh_cookies, method, url, headers, body).await
    }

    async fn send_with_session(
        cookies: &Cookies,
        method: &str,
        url: &str,
        mut headers: HashMap<String, Vec<String>>,
        body: String,
    ) -> Result<HttpResponse, ApiError> {
        Self::set_cookie_headers(&mut headers, cookies);

        // reading is allowed on the cookie alone, writing also wants the session's CSRF token
        if !method.eq_ignore_ascii_case("GET") {
            let token = Self::csrf_token(cookies).await?;
            headers.insert("X-CSRF-TOKEN".to_string(), vec![token]);
        }

        Ok(E::default().request(method, url, headers, body).await?)
    }

    /// The CSRF token of the session, which Untis ships in the config its pages bootstrap from
    /// rather than handing out through an endpoint, so it has to be read off one of them
    async fn csrf_token(cookies: &Cookies) -> Result<String, ApiError> {
        let school_name = Self::school_name()?;
        let url = format!(
            "https://{}.webuntis.com/WebUntis/?school={}",
            school_name, school_name,
        );

        let mut headers = HashMap::new();
        headers.insert("Cookie".to_string(), vec![cookies.to_header_value()]);
        let page = E::default().request("GET", &url, headers, String::new()).await?.body;

        extract_csrf_token(&page)
            .ok_or(ApiError::Authentication("No CSRF token in the WebUntis page".to_string()))
    }

    fn school_name() -> Result<String, ApiError> {
        Ok(Store::<E>::get_settings()?
            .ok_or(ApiError::Miscellaneous("Settings are empty".to_string()))?
            .untis_auth
            .school_identifier)
    }

    /// The headers of a session-authenticated request, for a caller that fetches a URL itself
    /// rather than through `session_request` (the native downloader does)
    pub fn session_headers() -> Result<HashMap<String, Vec<String>>, ApiError> {
        let cookies = Store::<E>::get_cookies()
            .ok_or(ApiError::Authentication("Not logged in".to_string()))?;

        let mut headers = HashMap::new();
        Self::set_cookie_headers(&mut headers, &cookies);
        Ok(headers)
    }

    /// The tenant travels in a header of its own next to the cookie, the way the web client sends it
    fn set_cookie_headers(headers: &mut HashMap<String, Vec<String>>, cookies: &Cookies) {
        headers.insert("Cookie".to_string(), vec![cookies.to_header_value()]);
        headers.insert("Tenant-Id".to_string(), vec![cookies.tenant_id.clone()]);
    }

    pub async fn authorized_request(
        method: &str,
        url: &str,
        mut headers: HashMap<String, Vec<String>>,
        body: String,
    ) -> Result<HttpResponse, ApiError> {
        let env = E::default();

        let token = Self::get_token().await?;
        headers.insert("Authorization".to_string(), vec![format!("Bearer {token}")]);
        let res = env.request(method, url, headers.clone(), body.clone()).await?;

        if res.body.contains("UNAUTHORIZED") || res.body.contains("TOKEN_EXPIRED") {
            Store::<E>::clear_token();
            Self::reauthenticate().await?;
            let new_token = Self::get_token().await?;
            headers.insert("Authorization".to_string(), vec![format!("Bearer {new_token}")]);
            return Ok(env.request(method, url, headers, body).await?);
        }

        Ok(res)
    }
}

/// When a JWT expires, in milliseconds since the epoch, read from its `exp` claim. `None` if the
/// token doesn't say, in which case it isn't trusted to still be valid.
fn token_expiry_ms(token: &str) -> Option<u64> {
    let payload = URL_SAFE_NO_PAD.decode(token.split('.').nth(1)?.trim_end_matches('=')).ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    claims["exp"].as_u64()?.checked_mul(1000)
}

/// Picks `"csrfToken":"..."` out of the config blob a WebUntis page bootstraps from
fn extract_csrf_token(page: &str) -> Option<String> {
    const KEY: &str = "\"csrfToken\":\"";

    let value = &page[page.find(KEY)? + KEY.len()..];
    Some(value[..value.find('"')?].to_string())
}

#[cfg(test)]
mod tests {
    use super::{extract_csrf_token, token_expiry_ms};

    #[test]
    fn reads_the_token_out_of_a_page() {
        let page = r#"<html><script>window.config = {"playgrounds":[],"hasPrivacyPolicy":false,"csrfHeader":"X-CSRF-TOKEN","csrfToken":"EG-kpVugFDsDJATk7a3Jb0PcSI8A9Srd","school":"htl-hl"}</script></html>"#;

        assert_eq!(extract_csrf_token(page).as_deref(), Some("EG-kpVugFDsDJATk7a3Jb0PcSI8A9Srd"));
    }

    #[test]
    fn reads_the_expiry_out_of_a_jwt() {
        // header and payload are base64url of {"alg":"HS256"} and {"exp":1700000000}
        let token = "eyJhbGciOiJIUzI1NiJ9.eyJleHAiOjE3MDAwMDAwMDB9.c2ln";
        assert_eq!(token_expiry_ms(token), Some(1_700_000_000_000));
    }

    #[test]
    fn has_no_expiry_without_one() {
        assert_eq!(token_expiry_ms("eyJhbGciOiJIUzI1NiJ9.e30.c2ln"), None);
        assert_eq!(token_expiry_ms("not a jwt"), None);
    }

    #[test]
    fn has_no_token_without_one() {
        assert_eq!(extract_csrf_token("<html>Session expired</html>"), None);
        assert_eq!(extract_csrf_token(r#"{"csrfToken":"#), None);
    }
}
