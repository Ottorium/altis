use crate::errors::ApiError;
use crate::persistence_manager::{Cookies, PersistenceManager};
use crate::request_proxy::{request_proxy, ProxyResponse};
use js_sys::Date;
use serde_json::json;
use std::collections::HashMap;
use totp_rs::{Algorithm, Secret, TOTP};

pub fn is_authenticated() -> bool {
    PersistenceManager::get_cookies().is_some()
}

pub async fn get_session_into_cookies(
    school_name: String,
    username: String,
    secret: String,
) -> Result<(), ApiError> {

    if school_name.is_empty() || username.is_empty() || secret.is_empty() {
        return Err(ApiError::Authentication("Credentials not set. ".to_string()))
    }

    let secret_bytes = Secret::Encoded(secret)
        .to_bytes()
        .map_err(|x| ApiError::Authentication(x.to_string()))?;
    let now_ms = Date::now() as u64;
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

    let response = request_proxy("POST", &login_url, HashMap::new(), body.to_string())
        .await
        .map_err(|x| ApiError::Network(x.to_string()))?;

    let response_json: serde_json::Value = serde_json::from_str(&response.body)
        .map_err(|x| ApiError::Parsing(x.to_string()))?;

    if let Some(error) = response_json.get("error") {
        let message = error["message"].as_str().unwrap_or("Unknown error");
        let code = error["code"].as_i64().unwrap_or(0);
        return Err(ApiError::Authentication(format!("API Error {}: {}", code, message)));
    }

    let mut jsessionid = None;
    let mut tenant_id = None;
    let mut school_name = None;

    for (_, values) in response.headers.iter().filter(|(k, _)| k.to_lowercase() == "set-cookie") {
        for val in values {
            let first_part = val.split(';').next().unwrap_or("");
            let mut parts = first_part.splitn(2, '=');
            let key = parts.next().unwrap_or("").trim();
            let value = parts.next().unwrap_or("").trim().to_string();

            match key {
                "JSESSIONID" => jsessionid = Some(value),
                "Tenant-Id" => tenant_id = Some(value),
                "schoolname" => school_name = Some(value),
                _ => {}
            }
        }
    }

    if let (Some(jsessionid), Some(tenant_id), Some(school_name)) = (jsessionid, tenant_id, school_name) {
        let cookies = Cookies { jsessionid, tenant_id, school_name_base32: school_name };
        PersistenceManager::save_cookies(&cookies).map_err(ApiError::Miscellaneous)?;
    }

    Ok(())
}

async fn get_token() -> Result<String, ApiError> {
    let cookies = PersistenceManager::get_cookies().ok_or(ApiError::Authentication("Could not get cookies".to_string()))?;
    let settings = PersistenceManager::get_settings()?.ok_or(ApiError::Miscellaneous("Settings are empty".to_string()))?;

    if settings.untis_auth.school_identifier.is_empty() || settings.untis_auth.user_identifier.is_empty() || settings.untis_auth.secret.is_empty() {
        return Err(ApiError::Authentication("Credentials not set. ".to_string()))
    }

    let url = format!("https://{}.webuntis.com/WebUntis/api/token/new", settings.untis_auth.school_identifier);
    let mut headers = HashMap::new();
    headers.insert("Cookie".to_string(), vec![cookies.to_header_value()]);
    Ok(request_proxy("GET", url.as_str(), headers, "".to_string()).await?.body)
}

pub async fn authorized_request(method: &str, url: &str, mut headers: HashMap<String, Vec<String>>, body: String) -> Result<ProxyResponse, ApiError> {
    let token = get_token().await?;
    headers.insert("Authorization".to_string(), vec![format!("Bearer {}", token)]);
    Ok(request_proxy(method, url, headers, body).await?)
}