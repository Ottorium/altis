use crate::data_models::clean_models::untis::{Class, WeekTimeTable};
use crate::untis::untis_week::Week;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use wasm_bindgen::JsCast;
use web_sys::{HtmlDocument, Storage};


pub type TimeTables = (HashMap<Class, WeekTimeTable>, Option<i32>);

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TimeTableCache {
    pub tables: HashMap<Week, (Option<NaiveDateTime>, TimeTables)>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub untis_auth: AuthSettings,
    pub b2e_auth: AuthSettings,
    pub visual_settings: VisualSettings,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct VisualSettings {
    pub force_ascii_timetable: bool,
}


#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AuthSettings {
    pub school_identifier: String,
    pub user_identifier: String,
    pub secret: String,
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct PersistenceManager {}

impl PersistenceManager {
    pub fn get_cookies() -> Option<Cookies> {
        let storage = Self::get_storage().ok()?;

        let jsessionid = storage.get_item("JSESSIONID").ok()??;
        let tenant_id = storage.get_item("Tenant-Id").ok()??;
        let school_name = storage.get_item("schoolname").ok()??;

        Some(Cookies {
            jsessionid,
            tenant_id,
            school_name_base32: school_name,
        })
    }

    pub fn save_cookies(cookies: &Cookies) -> Result<(), String> {
        let storage = Self::get_storage()?;

        storage
            .set_item("JSESSIONID", &cookies.jsessionid)
            .map_err(|_| "Failed to save JSESSIONID")?;

        storage
            .set_item("Tenant-Id", &cookies.tenant_id)
            .map_err(|_| "Failed to save Tenant-Id")?;

        storage
            .set_item("schoolname", &cookies.school_name_base32)
            .map_err(|_| "Failed to save schoolname")?;

        Ok(())
    }

    fn clear_cookies() {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        if let Ok(html_doc) = document.dyn_into::<HtmlDocument>() {
            let cookie_names = ["JSESSIONID", "Tenant-Id", "schoolname"];
            for name in cookie_names {
                let _ = html_doc.set_cookie(&format!("{}=; Max-Age=0; path=/; SameSite=Lax", name));
            }
        }
    }

    pub fn save_settings(settings: &Settings) -> Result<(), String> {
        if let Ok(Some(existing)) = Self::get_settings()
            && existing.untis_auth != settings.untis_auth {
                Self::clear_cookies();
            }

        let serialized =
            serde_json::to_string(settings).map_err(|e| format!("Serialization failed: {}", e))?;

        Self::get_storage()?
            .set_item("user_settings", &serialized)
            .map_err(|_| "Failed to write to localStorage")?;

        Ok(())
    }

    pub fn get_settings() -> Result<Option<Settings>, String> {
        let value = Self::get_storage()?
            .get_item("user_settings")
            .map_err(|_| "Error reading from localStorage")?;

        match value {
            Some(v) => Ok(Some(serde_json::from_str::<Settings>(&v).map_err(|e| format!("Failed to parse settings: {}", e))?)),
            None => Ok(None),
        }
    }

    pub fn save_timetables(tt: &TimeTableCache) -> Result<(), String> {
        let bytes = postcard::to_allocvec(tt)
            .map_err(|e| format!("Postcard failed: {}", e))?;

        let compressed = zstd::encode_all(&bytes[..], 3)
            .map_err(|e| format!("Compression failed: {}", e))?;

        let encoded = STANDARD.encode(compressed);

        Self::get_storage()?
            .set_item("cached_timetables", &encoded)
            .map_err(|_| "Failed to write to localStorage".to_string())?;

        Ok(())
    }

    pub fn get_timetables() -> Result<Option<TimeTableCache>, String> {
        let storage = Self::get_storage()?;
        let value = storage.get_item("cached_timetables")
            .map_err(|_| "Error reading from localStorage".to_string())?;

        match value {
            Some(v) => {
                let compressed_bytes = STANDARD.decode(v.trim())
                    .map_err(|e| format!("Base64 decode failed: {}", e))?;

                let mut decompressed = Vec::new();
                zstd::Decoder::new(&compressed_bytes[..])
                    .map_err(|e| e.to_string())?
                    .read_to_end(&mut decompressed)
                    .map_err(|e| format!("Decompression failed: {}", e))?;

                let decoded = postcard::from_bytes(&decompressed)
                    .map_err(|e| format!("Postcard failed: {}", e))?;

                Ok(Some(decoded))
            }
            None => Ok(None),
        }
    }

    pub fn clear_storage() -> Result<(), String> {
        Self::clear_cookies();
        Self::get_storage()?
            .clear()
            .map_err(|_| "Failed to clear localStorage".to_string())
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
