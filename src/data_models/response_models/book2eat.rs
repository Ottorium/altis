use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize, Eq)]
pub struct MenuResponse {
    pub data: Data,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize, Eq)]
pub struct Data {
    pub qr_code: String,
    pub day_from: String,
    pub day_to: String,
    pub menue: HashMap<String, Meal>,
    pub not_visible_days: Vec<i32>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize, Hash, Eq)]
pub struct Meal {
    pub name: String,
    pub date: String,
    pub typ_name: String,
    pub price: String,
}