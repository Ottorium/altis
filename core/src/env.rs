use std::collections::HashMap;

/// The parts of a request's response the Untis client cares about. Mirrors the shape the Tauri
/// `proxy` command returns, since that's what the frontend gets its responses from.
#[derive(Debug, Clone, Default)]
pub struct HttpResponse {
    pub headers: HashMap<String, Vec<String>>,
    pub body: String,
}

impl HttpResponse {
    /// The values of a header, matched case-insensitively (Untis sends `Set-Cookie`, but nothing
    /// guarantees the casing survives the trip through the platform's HTTP stack)
    pub fn header(&self, name: &str) -> impl Iterator<Item = &String> {
        self.headers
            .iter()
            .filter(move |(key, _)| key.eq_ignore_ascii_case(name))
            .flat_map(|(_, values)| values)
    }
}

/// Everything platform-specific the shared code needs: a key-value store, an HTTP client that
/// isn't subject to CORS, the wall clock, and a way to show a notification.
///
/// The frontend implements this on top of localStorage, the Tauri `proxy` command and the
/// notification plugin; the Android background poller on top of a JSON file, reqwest and a JNI
/// call into the foreground service. `Default` is required so the shared types can be used
/// through a plain type alias (`type UntisClient = untis::UntisClient<WebEnv>`) and keep their
/// `UntisClient::new()`-style constructors.
#[allow(async_fn_in_trait)]
pub trait Env: Default {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
    fn remove(&self, key: &str);

    /// Milliseconds since the Unix epoch. `chrono` would do on every platform we support, but the
    /// frontend already has `js_sys::Date` and there's no reason to pull a second clock into WASM.
    fn now_ms(&self) -> u64;

    async fn request(
        &self,
        method: &str,
        url: &str,
        headers: HashMap<String, Vec<String>>,
        body: String,
    ) -> Result<HttpResponse, String>;

    /// Shows a native notification. Best effort: a platform that can't show one just does nothing.
    async fn notify(&self, title: &str, body: &str);
}
