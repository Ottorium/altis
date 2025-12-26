use crate::persistence_manager::PersistenceManager;
use crate::request_proxy::request_proxy;
use js_sys::Date;
use serde_json::json;
use std::collections::HashMap;
use totp_rs::{Algorithm, Secret, TOTP};
use wasm_bindgen::JsCast;
use web_sys::HtmlDocument;

#[derive(Default, Clone, PartialEq, Debug)]
pub struct UntisClient {}

impl UntisClient {
    pub fn is_authenticated() -> bool {
        PersistenceManager::get_auth_cookies()
            .split("; ")
            .any(|cookie| cookie.starts_with("JSESSIONID="))
    }

    pub async fn get_session_into_cookies(
        school_name: String,
        username: String,
        secret: String,
    ) -> Result<(), String> {
        let secret_bytes = Secret::Encoded(secret)
            .to_bytes()
            .map_err(|x| x.to_string())?;
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

        // CORS disallows this request as well as the Set-Cookie header, so we need a proxy
        let response = request_proxy(&login_url, HashMap::new(), body)
            .await
            .map_err(|x| x.to_string())?;

        let response_json = response.body;
        if let Some(error) = response_json.get("error") {
            let message = error["message"].as_str().unwrap_or("Unknown error");
            let code = error["code"].as_i64().unwrap_or(0);
            return Err(format!("API Error {}: {}", code, message));
        }

        let cookie_strings: Vec<String> = response
            .headers
            .iter()
            .filter(|(k, _)| k.to_lowercase() == "set-cookie")
            .flat_map(|(_, values)| values.iter())
            .filter_map(|s| s.split(';').next())
            .map(|s| s.to_string())
            .collect();

        if let Ok(html_doc) = web_sys::window()
            .ok_or("Could not get window")?
            .document()
            .ok_or("Could not get Document")?
            .dyn_into::<HtmlDocument>()
        {
            for cookie_part in cookie_strings {
                let cookie_str = format!("{}; Path=/; SameSite=Strict", cookie_part);
                let _ = html_doc.set_cookie(&cookie_str);
            }
        }

        Ok(())
    }
}
