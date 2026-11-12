//! Logging in to WebUntis and keeping the session going.
//!
//! Untis has two ways of knowing who is asking, and both are needed: the newer REST API takes a
//! bearer token, which is handed out for the session cookies; the older `classreg` API only knows
//! the cookies themselves, plus a CSRF token for anything that writes.

use super::webuntis_url;
use crate::env::{header, Env, Headers, HttpResponse};
use crate::errors::ApiError;
use crate::settings::AuthSettings;
use crate::store::Store;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::json;
use std::marker::PhantomData;
use totp_rs::{Algorithm, Secret, TOTP};

/// How long before its expiry a cached token is replaced, so it can't run out mid-request
const TOKEN_EXPIRY_MARGIN_MS: u64 = 60_000;

const TOTP_DIGITS: usize = 6;
const TOTP_STEP_SECONDS: u64 = 30;
/// how many steps to either side of the current one Untis would accept
const TOTP_SKEW: u8 = 1;

/// The cookies an Untis session consists of
#[derive(Default, Clone, PartialEq, Debug)]
pub struct Cookies {
    pub jsessionid: String,
    pub tenant_id: String,
    pub school_name_base32: String,
}

impl Cookies {
    pub fn to_header_value(&self) -> String {
        format!("JSESSIONID={}; Tenant-Id={}; schoolname={}", self.jsessionid, self.tenant_id, self.school_name_base32)
    }

    /// Picks the session's cookies out of a login response, `None` if one of them is missing
    fn from_response(response: &HttpResponse) -> Option<Self> {
        // a cookie sent twice counts as it was sent last
        let cookie = |name: &str| {
            response
                .header("set-cookie")
                .filter_map(|cookie| {
                    let assignment = cookie.split(';').next().unwrap_or_default();
                    let (key, value) = assignment.split_once('=').unwrap_or((assignment, ""));
                    (key.trim() == name).then(|| value.trim().to_string())
                })
                .last()
        };

        Some(Self {
            jsessionid: cookie("JSESSIONID")?,
            tenant_id: cookie("Tenant-Id")?,
            school_name_base32: cookie("schoolname")?,
        })
    }
}

pub struct AuthHelper<E: Env>(PhantomData<E>);

impl<E: Env> AuthHelper<E> {
    pub fn is_authenticated() -> bool {
        Store::<E>::get_cookies().is_some()
    }

    /// Logs in with a one-time code made from the secret, the way the official app does, and
    /// stores the session's cookies
    pub async fn authenticate(auth: &AuthSettings) -> Result<(), ApiError> {
        if !auth.is_complete() {
            return Err(ApiError::Authentication("Credentials not set".to_string()));
        }

        let env = E::default();
        let now_ms = env.now_ms();
        let school = &auth.school_identifier;
        let url = webuntis_url(school, &format!("jsonrpc_intern.do?m=getUserData2017&school={school}&v=i2.2"));
        let body = json!({
            "id": "opensource-client",
            "method": "getUserData2017",
            "params": [{"auth": {"clientTime": now_ms, "user": auth.user_identifier, "otp": one_time_code(&auth.secret, now_ms)?}}],
            "jsonrpc": "2.0"
        });

        let response = env.request("POST", &url, Headers::new(), body.to_string()).await.map_err(ApiError::Network)?;
        check_login_error(&response.body)?;

        if let Some(cookies) = Cookies::from_response(&response) {
            Store::<E>::save_cookies(&cookies).map_err(ApiError::Miscellaneous)?;
        }
        Ok(())
    }

    fn stored_auth() -> Result<AuthSettings, ApiError> {
        Ok(Store::<E>::get_settings()?.ok_or(ApiError::Miscellaneous("Settings are empty".to_string()))?.untis_auth)
    }

    /// Authenticates with the credentials from the settings, returning the fresh cookies
    async fn reauthenticate() -> Result<Cookies, ApiError> {
        Self::authenticate(&Self::stored_auth()?).await?;

        Store::<E>::get_cookies().ok_or(ApiError::Authentication("Could not get cookies after authenticating".to_string()))
    }

    /// The session's cookies, logging in first if there is no session
    async fn cookies() -> Result<Cookies, ApiError> {
        match Store::<E>::get_cookies() {
            Some(cookies) => Ok(cookies),
            None => Self::reauthenticate().await,
        }
    }

    async fn request_token(cookies: &Cookies, school: &str) -> Result<String, ApiError> {
        let headers = Headers::from([header("Cookie", cookies.to_header_value())]);
        let response = E::default().request("GET", &webuntis_url(school, "api/token/new"), headers, String::new()).await?;
        Ok(response.body)
    }

    /// The bearer token of the session: the cached one while it is good, a new one otherwise
    async fn token() -> Result<String, ApiError> {
        let auth = Self::stored_auth()?;
        if !auth.is_complete() {
            return Err(ApiError::Authentication("Credentials not set".to_string()));
        }
        let school = &auth.school_identifier;

        // a token is good for many requests, fetching one per request doubles every round trip
        let now_ms = E::default().now_ms();
        if let Some(token) = Store::<E>::get_token()
            && token_expiry_ms(&token).is_some_and(|expiry| expiry > now_ms + TOKEN_EXPIRY_MARGIN_MS) {
            return Ok(token);
        }

        let body = Self::request_token(&Self::cookies().await?, school).await?;
        if let Some(token) = as_jwt(&body) {
            return Self::cache_token(token);
        }

        // no token means the session is gone, so it is asked for once more with a new one
        let retry_body = Self::request_token(&Self::reauthenticate().await?, school).await?;
        let token = as_jwt(&retry_body)
            .ok_or_else(|| ApiError::Authentication(format!("Failed to obtain JWT token: {}", retry_body.trim())))?;
        Self::cache_token(token)
    }

    fn cache_token(token: String) -> Result<String, ApiError> {
        Store::<E>::save_token(&token).map_err(ApiError::Miscellaneous)?;
        Ok(token)
    }

    /// Performs a request authenticated with the bearer token, the way the REST API wants it
    pub async fn authorized_request(method: &str, url: &str, headers: Headers, body: String) -> Result<HttpResponse, ApiError> {
        let env = E::default();
        let send = |token: String| {
            let mut headers = headers.clone();
            headers.extend([header("Authorization", format!("Bearer {token}"))]);
            env.request(method, url, headers, body.clone())
        };

        let response = send(Self::token().await?).await?;
        if !(response.body.contains("UNAUTHORIZED") || response.body.contains("TOKEN_EXPIRED")) {
            return Ok(response);
        }

        Store::<E>::clear_token();
        Self::reauthenticate().await?;
        Ok(send(Self::token().await?).await?)
    }

    /// Performs a request authenticated with the session cookies. The classreg API is older than
    /// the token one and only knows the session, so a bearer token is no use to it.
    pub async fn session_request(method: &str, url: &str, headers: Headers, body: String) -> Result<HttpResponse, ApiError> {
        let response = Self::send_with_session(&Self::cookies().await?, method, url, headers.clone(), body.clone()).await?;

        // a session that's gone gets the login page rather than an error, so anything that isn't
        // JSON means we have to log in again instead of that the request itself failed
        if response.body.trim_start().starts_with('{') {
            return Ok(response);
        }

        Self::send_with_session(&Self::reauthenticate().await?, method, url, headers, body).await
    }

    async fn send_with_session(cookies: &Cookies, method: &str, url: &str, mut headers: Headers, body: String) -> Result<HttpResponse, ApiError> {
        headers.extend(session_headers(cookies));

        // reading is allowed on the cookie alone, writing also wants the session's CSRF token
        if !method.eq_ignore_ascii_case("GET") {
            headers.extend([header("X-CSRF-TOKEN", Self::csrf_token(cookies).await?)]);
        }

        Ok(E::default().request(method, url, headers, body).await?)
    }

    /// The CSRF token of the session, which Untis ships in the config its pages bootstrap from
    /// rather than handing out through an endpoint, so it has to be read off one of them
    async fn csrf_token(cookies: &Cookies) -> Result<String, ApiError> {
        let school = Self::stored_auth()?.school_identifier;
        let headers = Headers::from([header("Cookie", cookies.to_header_value())]);
        let page = E::default().request("GET", &webuntis_url(&school, &format!("?school={school}")), headers, String::new()).await?.body;

        extract_csrf_token(&page).ok_or(ApiError::Authentication("No CSRF token in the WebUntis page".to_string()))
    }

    /// The headers of a session-authenticated request, for a caller that fetches a URL itself
    /// rather than through `session_request` (the native downloader does)
    pub fn session_headers() -> Result<Headers, ApiError> {
        let cookies = Store::<E>::get_cookies().ok_or(ApiError::Authentication("Not logged in".to_string()))?;
        Ok(session_headers(&cookies))
    }
}

/// The tenant travels in a header of its own next to the cookie, the way the web client sends it
fn session_headers(cookies: &Cookies) -> Headers {
    Headers::from([header("Cookie", cookies.to_header_value()), header("Tenant-Id", cookies.tenant_id.clone())])
}

/// The code Untis expects along with the user name: a TOTP of the secret it hands out for apps
fn one_time_code(secret: &str, now_ms: u64) -> Result<String, ApiError> {
    let secret = Secret::Encoded(secret.to_string()).to_bytes().map_err(|e| ApiError::Authentication(e.to_string()))?;
    let totp = TOTP::new_unchecked(Algorithm::SHA1, TOTP_DIGITS, TOTP_SKEW, TOTP_STEP_SECONDS, secret);
    Ok(totp.generate(now_ms / 1000))
}

/// Untis answers a refused login with a JSON-RPC error rather than a status code
fn check_login_error(body: &str) -> Result<(), ApiError> {
    let response: serde_json::Value = serde_json::from_str(body).map_err(|e| ApiError::Parsing(e.to_string()))?;
    let Some(error) = response.get("error") else { return Ok(()) };

    let message = error["message"].as_str().unwrap_or("Unknown error");
    let code = error["code"].as_i64().unwrap_or(0);
    Err(ApiError::Authentication(format!("API Error {code}: {message}")))
}

/// A JWT is only accepted if it really is one: Untis answers an expired session with a JSON
/// error body rather than a status code, so anything that isn't three dot-separated parts
/// means the session is gone and we have to log in again.
fn as_jwt(body: &str) -> Option<String> {
    let body = body.trim();
    (body.split('.').count() == 3 && !body.starts_with('{')).then(|| body.to_string())
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
    use super::*;
    use crate::settings::Settings;
    use crate::test_env::{Request, TestEnv};
    use futures::executor::block_on;

    type Auth = AuthHelper<TestEnv>;

    /// header and payload are base64url of {"alg":"HS256"} and {"exp":1700000000}
    const TOKEN: &str = "eyJhbGciOiJIUzI1NiJ9.eyJleHAiOjE3MDAwMDAwMDB9.c2ln";
    const TOKEN_EXPIRY_MS: u64 = 1_700_000_000_000;

    fn auth() -> AuthSettings {
        AuthSettings { school_identifier: "school".to_string(), user_identifier: "max".to_string(), secret: "JBSWY3DPEHPK3PXP".to_string() }
    }

    fn logged_out() {
        TestEnv::reset();
        Store::<TestEnv>::save_settings(&Settings { untis_auth: auth(), ..Default::default() }).unwrap();
    }

    fn response(body: &str) -> Result<HttpResponse, String> {
        Ok(HttpResponse { headers: Headers::new(), body: body.to_string() })
    }

    /// An Untis that logs anyone in, hands out [`TOKEN`] and answers everything else with `{}`
    fn untis(request: &Request) -> Result<HttpResponse, String> {
        if request.url.contains("jsonrpc_intern.do") {
            let cookies = ["JSESSIONID=ABC; Path=/WebUntis; Secure", "Tenant-Id=42; Path=/", "schoolname=_c2Nob29s; Max-Age=1"];
            return Ok(HttpResponse {
                headers: Headers::from([("Set-Cookie".to_string(), cookies.map(String::from).to_vec())]),
                body: r#"{"jsonrpc":"2.0","result":{}}"#.to_string(),
            });
        }
        if request.url.ends_with("api/token/new") {
            return response(TOKEN);
        }
        if request.url.ends_with("?school=school") {
            return response(r#"<script>window.config = {"csrfToken":"CSRF"}</script>"#);
        }
        response("{}")
    }

    fn urls() -> Vec<String> {
        TestEnv::requests().into_iter().map(|request| request.url).collect()
    }

    #[test]
    fn reads_the_token_out_of_a_page() {
        let page = r#"<html><script>window.config = {"playgrounds":[],"hasPrivacyPolicy":false,"csrfHeader":"X-CSRF-TOKEN","csrfToken":"EG-kpVugFDsDJATk7a3Jb0PcSI8A9Srd","school":"htl-hl"}</script></html>"#;

        assert_eq!(extract_csrf_token(page).as_deref(), Some("EG-kpVugFDsDJATk7a3Jb0PcSI8A9Srd"));
    }

    #[test]
    fn reads_the_expiry_out_of_a_jwt() {
        assert_eq!(token_expiry_ms(TOKEN), Some(TOKEN_EXPIRY_MS));
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

    #[test]
    fn only_a_jwt_counts_as_a_token() {
        assert_eq!(as_jwt(" a.b.c\n").as_deref(), Some("a.b.c"));
        assert_eq!(as_jwt(r#"{"error":"session.expired.now"}"#), None);
        assert_eq!(as_jwt("<html>"), None);
    }

    /// RFC 6238's test secret, to make sure the code is the one any authenticator would show
    #[test]
    fn makes_the_one_time_code_an_authenticator_would() {
        // base32 of "12345678901234567890"
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        assert_eq!(one_time_code(secret, 59_000).unwrap(), "287082");
        assert_eq!(one_time_code(secret, 1_111_111_109_000).unwrap(), "081804");
        assert!(one_time_code("not base32!", 0).is_err());
    }

    #[test]
    fn logging_in_stores_the_session_cookies() {
        logged_out();
        TestEnv::respond_with(untis);

        block_on(Auth::authenticate(&auth())).unwrap();

        let cookies = Store::<TestEnv>::get_cookies().unwrap();
        assert_eq!(cookies.to_header_value(), "JSESSIONID=ABC; Tenant-Id=42; schoolname=_c2Nob29s");
        assert!(Auth::is_authenticated());

        let login = &TestEnv::requests()[0];
        assert_eq!(login.method, "POST");
        assert_eq!(login.url, "https://school.webuntis.com/WebUntis/jsonrpc_intern.do?m=getUserData2017&school=school&v=i2.2");
        assert!(login.body.contains(r#""user":"max""#));
    }

    #[test]
    fn a_refused_login_says_why() {
        logged_out();
        TestEnv::respond_with_body(r#"{"error":{"code":-8504,"message":"bad credentials"}}"#);

        let error = block_on(Auth::authenticate(&auth())).unwrap_err();
        assert_eq!(error.to_string(), "Auth Error: API Error -8504: bad credentials");
        assert!(!Auth::is_authenticated());
    }

    #[test]
    fn incomplete_credentials_are_not_even_sent() {
        logged_out();

        let error = block_on(Auth::authenticate(&AuthSettings::default())).unwrap_err();
        assert_eq!(error.to_string(), "Auth Error: Credentials not set");
        assert!(TestEnv::requests().is_empty());
    }

    #[test]
    fn a_request_logs_in_gets_a_token_and_reuses_it() {
        logged_out();
        TestEnv::respond_with(untis);
        TestEnv::set_now_ms(TOKEN_EXPIRY_MS - 10 * 60_000);

        block_on(Auth::authorized_request("GET", "https://school.webuntis.com/data", Headers::new(), String::new())).unwrap();
        assert_eq!(urls().len(), 3);
        let request = TestEnv::requests().pop().unwrap();
        assert_eq!(request.headers["Authorization"], vec![format!("Bearer {TOKEN}")]);

        // the second request has both a session and a token to go on
        block_on(Auth::authorized_request("GET", "https://school.webuntis.com/data", Headers::new(), String::new())).unwrap();
        assert_eq!(urls().len(), 4);
    }

    #[test]
    fn a_token_about_to_expire_is_replaced() {
        logged_out();
        TestEnv::respond_with(untis);
        TestEnv::set_now_ms(TOKEN_EXPIRY_MS - 10 * 60_000);
        block_on(Auth::authorized_request("GET", "https://school.webuntis.com/data", Headers::new(), String::new())).unwrap();

        TestEnv::set_now_ms(TOKEN_EXPIRY_MS - TOKEN_EXPIRY_MARGIN_MS);
        block_on(Auth::authorized_request("GET", "https://school.webuntis.com/data", Headers::new(), String::new())).unwrap();
        assert_eq!(urls()[3], "https://school.webuntis.com/WebUntis/api/token/new");
    }

    #[test]
    fn a_session_request_sends_the_cookies_and_only_writes_fetch_the_csrf_token() {
        logged_out();
        TestEnv::respond_with(untis);

        block_on(Auth::session_request("GET", "https://school.webuntis.com/read", Headers::new(), String::new())).unwrap();
        let read = TestEnv::requests().pop().unwrap();
        assert_eq!(read.headers["Cookie"], vec!["JSESSIONID=ABC; Tenant-Id=42; schoolname=_c2Nob29s".to_string()]);
        assert_eq!(read.headers["Tenant-Id"], vec!["42".to_string()]);
        assert!(!read.headers.contains_key("X-CSRF-TOKEN"));

        block_on(Auth::session_request("POST", "https://school.webuntis.com/write", Headers::new(), "{}".to_string())).unwrap();
        let write = TestEnv::requests().pop().unwrap();
        assert_eq!(write.headers["X-CSRF-TOKEN"], vec!["CSRF".to_string()]);
    }

    /// Untis answers a request on a dead session with its login page, not with an error
    #[test]
    fn a_session_that_is_gone_is_replaced_and_the_request_repeated() {
        logged_out();
        Store::<TestEnv>::save_cookies(&Cookies { jsessionid: "OLD".to_string(), tenant_id: "42".to_string(), school_name_base32: "_x".to_string() }).unwrap();
        TestEnv::respond_with(|request| {
            let stale = request.headers.get("Cookie").is_some_and(|cookie| cookie[0].contains("JSESSIONID=OLD"));
            if stale { response("<html>Login</html>") } else { untis(request) }
        });

        let answer = block_on(Auth::session_request("GET", "https://school.webuntis.com/read", Headers::new(), String::new())).unwrap();
        assert_eq!(answer.body, "{}");
        assert_eq!(urls().len(), 3);
        assert_eq!(Store::<TestEnv>::get_cookies().unwrap().jsessionid, "ABC");
    }
}
