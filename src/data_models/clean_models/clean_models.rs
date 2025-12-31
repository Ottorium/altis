use serde::Serialize;

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize)]
pub struct Class {
    pub id: i32,
    pub name: String,
    pub class_teacher: Option<Teacher>,
    pub department: Department,
}

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize)]
pub struct Department {
    pub id: i32,
    pub short_name: String,
    pub long_name: String,
    pub display_name: String,
}

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize)]
pub struct Teacher {
    pub id: i32,
    pub short_name: String,
    pub long_name: String,
    pub display_name: String,
}