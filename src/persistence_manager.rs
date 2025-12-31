use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{HtmlDocument, Storage};

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub school_name: String,
    pub username: String,
    pub auth_secret: String,
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct PersistenceManager {}

impl PersistenceManager {
    fn get_all_cookies() -> String {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        if let Ok(html_doc) = document.dyn_into::<HtmlDocument>() {
            if let Ok(cookies) = html_doc.cookie() {
                return cookies;
            }
        }
        String::new()
    }

    pub fn get_cookies() -> Option<Cookies> {
        let raw_cookies = Self::get_all_cookies();

        let mut jsessionid = None;
        let mut tenant_id = None;
        let mut school_name = None;

        for cookie in raw_cookies.split(';') {
            let mut parts = cookie.splitn(2, '=');
            let key = parts.next().unwrap_or("").trim();
            let val = parts.next().unwrap_or("").trim().to_string();

            match key {
                "JSESSIONID" => jsessionid = Some(val),
                "Tenant-Id" => tenant_id = Some(val),
                "schoolname" => school_name = Some(val),
                _ => {}
            }
        }

        match (jsessionid, tenant_id, school_name) {
            (Some(jsessionid), Some(tenant_id), Some(school_name)) => {
                Some(Cookies {
                    jsessionid,
                    tenant_id,
                    school_name_base32: school_name,
                })
            }
            _ => None,
        }
    }

    pub fn save_settings(settings: &Settings) -> Result<(), String> {
        let serialized =
            serde_json::to_string(settings).map_err(|e| format!("Serialization failed: {}", e))?;

        Self::get_storage()?
            .set_item("user_settings", &serialized)
            .map_err(|_| "Failed to write to localStorage (storage might be full)")?;

        Ok(())
    }

    pub fn get_settings() -> Result<Settings, String> {
        let value = Self::get_storage()?
            .get_item("user_settings")
            .map_err(|_| "Error reading from localStorage")?
            .ok_or("No settings found in storage")?;

        let settings = serde_json::from_str::<Settings>(&value)
            .map_err(|e| format!("Failed to parse settings: {}", e))?;

        Ok(settings)
    }

    fn get_storage() -> Result<Storage, String> {
        let window = web_sys::window().ok_or("No global window found")?;
        let storage = window
            .local_storage()
            .map_err(|_| "LocalStorage access denied (check permissions)")?
            .ok_or("LocalStorage is not available in this environment")?;
        Ok(storage)
    }
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Cookies {
    pub jsessionid: String,
    pub tenant_id: String,
    pub school_name_base32: String,
}

impl Cookies {
    pub fn to_header_value(&self) -> String {
        format!(
            "JSESSIONID={}; Tenant-Id={}; schoolname={}",
            self.jsessionid, self.tenant_id, self.school_name_base32
        )
    }
}
