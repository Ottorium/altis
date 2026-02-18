use crate::data_models::response_models::book2eat::MenuResponse;
use crate::errors::ApiError;
use crate::persistence_manager::PersistenceManager;
use crate::request_proxy::request_proxy;
use crate::untis::untis_week::Week;
use serde_json::Value;
use std::collections::HashMap;

const APP_VERIFICATION_HASH: &str =
    "g!K(g43wVIV*(Jy!y%3re3Th(4I*2Z8L&#up9mcKLGAhfS@ERY4(kKXM2)6RqSvR";

pub async fn get_b2e_token() -> Result<(String, String), ApiError> {
    let s = PersistenceManager::get_settings()?
        .filter(|s| !s.b2e_auth.school_identifier.is_empty() && !s.b2e_auth.secret.is_empty())
        .ok_or_else(|| ApiError::Authentication("B2E Credentials not set".into()))?;

    let params = [
        ("mail", &s.b2e_auth.user_identifier),
        ("password", &s.b2e_auth.secret),
        ("canteen_ID", &s.b2e_auth.school_identifier),
        ("app_verification_hash", &APP_VERIFICATION_HASH.to_string()),
    ];

    let body =
        serde_urlencoded::to_string(params).map_err(|e| ApiError::Miscellaneous(e.to_string()))?;

    let headers = HashMap::from([(
        "Content-Type".into(),
        vec!["application/x-www-form-urlencoded; charset=utf-8".into()],
    )]);

    let res = request_proxy(
        "POST",
        "https://tech-2.eu/app/book2eat.eu/api/v2.0.25/c/account_login.php",
        headers,
        body,
    )
    .await?;

    let json: Value =
        serde_json::from_str(&res.body).map_err(|e| ApiError::Parsing(e.to_string()))?;

    let token = json["data"]["token"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| ApiError::Authentication("Token not found".into()))?;

    let user_id = json["data"]["user_ID"]
        .as_i64()
        .map(|id| id.to_string())
        .ok_or_else(|| ApiError::Authentication("user_ID not found".into()))?;
    Ok((user_id, token))
}

pub async fn get_menu(week: Week, user_id: String, token: String) -> Result<MenuResponse, ApiError> {
    let s = PersistenceManager::get_settings()?
        .filter(|s| !s.b2e_auth.school_identifier.is_empty())
        .ok_or_else(|| ApiError::Authentication("B2E Credentials not set".into()))?;


    let params = [
        ("user_ID", &user_id),
        ("token", &token),
        ("canteen_ID", &s.b2e_auth.school_identifier),
        ("app_verification_hash", &APP_VERIFICATION_HASH.to_string()),
        ("day_from", &week.start.to_string()),
        ("day_to", &week.end.to_string()),
    ];

    let body =
        serde_urlencoded::to_string(params).map_err(|e| ApiError::Miscellaneous(e.to_string()))?;

    let headers = HashMap::from([(
        "Content-Type".into(),
        vec!["application/x-www-form-urlencoded; charset=utf-8".into()],
    )]);

    let res = request_proxy(
        "POST",
        "https://tech-2.eu/app/book2eat.eu/api/v2.0.25/c/home.php",
        headers,
        body,
    )
        .await?;

    let menu: MenuResponse =
        serde_json::from_str(&res.body).map_err(|e| ApiError::Parsing(format!("Serialization error: {}", e)))?;

    Ok(menu)
}
