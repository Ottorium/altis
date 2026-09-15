use crate::env::{Env, HttpResponse};
use crate::errors::ApiError;
use crate::settings::Cookies;
use crate::store::Store;
use serde_json::json;
use std::collections::HashMap;
use std::marker::PhantomData;
use totp_rs::{Algorithm, Secret, TOTP};

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

        let cookies = match Store::<E>::get_cookies() {
            Some(cookies) => cookies,
            None => Self::reauthenticate().await?,
        };

        let body = Self::request_token(&cookies, school_name).await?;
        if let Some(token) = Self::as_jwt(&body) {
            return Ok(token);
        }

        let fresh_cookies = Self::reauthenticate().await?;
        let retry_body = Self::request_token(&fresh_cookies, school_name).await?;
        Self::as_jwt(&retry_body).ok_or_else(|| {
            ApiError::Authentication(format!("Failed to obtain JWT token: {}", retry_body.trim()))
        })
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
            Self::reauthenticate().await?;
            let new_token = Self::get_token().await?;
            headers.insert("Authorization".to_string(), vec![format!("Bearer {new_token}")]);
            return Ok(env.request(method, url, headers, body).await?);
        }

        Ok(res)
    }
}
