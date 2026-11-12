//! The canteen's API. It has nothing to do with Untis: its own login, its own server.

use crate::{native, storage};
use altis_core::env::{header, Headers};
use altis_core::errors::ApiError;
use altis_core::models::book2eat::MenuResponse;
use altis_core::settings::AuthSettings;
use altis_core::untis::week::Week;
use serde_json::Value;

const API_URL: &str = "https://tech-2.eu/app/book2eat.eu/api/v2.0.25/c";

/// What the canteen's own app identifies itself with
const APP_VERIFICATION_HASH: &str = "g!K(g43wVIV*(Jy!y%3re3Th(4I*2Z8L&#up9mcKLGAhfS@ERY4(kKXM2)6RqSvR";

/// A login at the canteen
struct Session {
    user_id: String,
    token: String,
}

fn stored_auth() -> Result<AuthSettings, ApiError> {
    storage::settings()?
        .map(|settings| settings.b2e_auth)
        .filter(|auth| !auth.school_identifier.is_empty() && !auth.secret.is_empty())
        .ok_or_else(|| ApiError::Authentication("B2E Credentials not set".into()))
}

/// Posts a form to an endpoint of the API and returns the response's body
async fn post(endpoint: &str, form: &[(&str, &str)]) -> Result<String, ApiError> {
    let body = serde_urlencoded::to_string(form).map_err(|e| ApiError::Miscellaneous(e.to_string()))?;
    let headers = Headers::from([header("Content-Type", "application/x-www-form-urlencoded; charset=utf-8")]);

    Ok(native::proxy_request("POST", &format!("{API_URL}/{endpoint}"), &headers, &body).await?.body)
}

async fn login(auth: &AuthSettings) -> Result<Session, ApiError> {
    let form = [
        ("mail", auth.user_identifier.as_str()),
        ("password", auth.secret.as_str()),
        ("canteen_ID", auth.school_identifier.as_str()),
        ("app_verification_hash", APP_VERIFICATION_HASH),
    ];
    let response: Value = serde_json::from_str(&post("account_login.php", &form).await?).map_err(|e| ApiError::Parsing(e.to_string()))?;

    let token = response["data"]["token"].as_str().ok_or_else(|| ApiError::Authentication("Token not found".into()))?;
    let user_id = response["data"]["user_ID"].as_i64().ok_or_else(|| ApiError::Authentication("user_ID not found".into()))?;
    Ok(Session { user_id: user_id.to_string(), token: token.to_string() })
}

/// The week's menu, along with the QR code the canteen scans
pub async fn get_menu(week: &Week) -> Result<MenuResponse, ApiError> {
    let auth = stored_auth()?;
    let session = login(&auth).await?;

    let (day_from, day_to) = (week.start.to_string(), week.end.to_string());
    let form = [
        ("user_ID", session.user_id.as_str()),
        ("token", session.token.as_str()),
        ("canteen_ID", auth.school_identifier.as_str()),
        ("app_verification_hash", APP_VERIFICATION_HASH),
        ("day_from", day_from.as_str()),
        ("day_to", day_to.as_str()),
    ];

    serde_json::from_str(&post("home.php", &form).await?).map_err(|e| ApiError::Parsing(format!("Serialization error: {e}")))
}
