mod auth;
pub mod client;
pub mod week;

pub use auth::Cookies;

/// The address of something on a school's WebUntis server
fn webuntis_url(school: &str, path: &str) -> String {
    format!("https://{school}.webuntis.com/WebUntis/{path}")
}
