//! Tells the user when a newer release has been published.
//!
//! GitHub is asked at most once a day and everything in between runs off that last answer, so
//! opening the app never waits on the network. The banner itself is frontend (see
//! `components::update_banner`); the comparing and the bookkeeping live here, where they can be
//! tested without a browser.

use crate::env::Env;
use crate::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Drafts and prereleases are not included here, so this only ever sees what was really published
const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/Ottorium/altis/releases/latest";

/// Where the banner sends anyone who wants the new version
pub const RELEASES_PAGE: &str = "https://github.com/Ottorium/altis/releases";

const DAY_MS: u64 = 24 * 60 * 60 * 1000;

/// How long hiding the banner hides it for
const DISMISS_DAYS: u64 = 10;

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct UpdateState {
    /// when GitHub was last asked, so it is asked once a day rather than on every start
    #[serde(default)]
    pub last_checked_ms: u64,
    /// the newest release seen, whether or not it is newer than what is running
    #[serde(default)]
    pub latest_version: String,
    /// the release the banner was last hidden for. Kept alongside the deadline so that a version
    /// newer than it still shows up, rather than being swallowed by the remaining ten days.
    #[serde(default)]
    pub dismissed_version: String,
    #[serde(default)]
    pub dismissed_until_ms: u64,
}

/// Parses a release tag as leniently as the tags actually are: `0.2.1`, `0.2` and a stray `v0.2`
/// all work, and a missing part counts as zero, so `0.2` and `0.2.0` are one and the same release.
fn parse_version(raw: &str) -> Option<[u64; 3]> {
    let raw = raw.trim().trim_start_matches(['v', 'V']);
    if raw.is_empty() {
        return None;
    }

    let mut parts = [0u64; 3];
    for (index, part) in raw.split('.').enumerate() {
        if index >= parts.len() {
            return None;
        }
        parts[index] = part.trim().parse().ok()?;
    }
    Some(parts)
}

/// Whether `candidate` is a release newer than `current`. Anything that doesn't parse counts as
/// "nothing newer": a tag that can't be compared is not worth nagging anyone about.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

#[derive(Deserialize)]
struct Release {
    #[serde(default)]
    tag_name: String,
}

async fn fetch_latest<E: Env>() -> Option<String> {
    let mut headers = HashMap::new();
    // GitHub answers a request without a User-Agent with 403 rather than the release
    headers.insert("User-Agent".to_string(), vec!["altis".to_string()]);
    headers.insert("Accept".to_string(), vec!["application/vnd.github+json".to_string()]);

    let response = E::default()
        .request("GET", LATEST_RELEASE_URL, headers, String::new())
        .await
        .ok()?;

    let release: Release = serde_json::from_str(&response.body).ok()?;
    (!release.tag_name.is_empty()).then_some(release.tag_name)
}

/// The release worth showing, given what was last seen and what the user hid
fn available(state: &UpdateState, current: &str, now: u64) -> Option<String> {
    if !is_newer(&state.latest_version, current) {
        return None;
    }
    if state.dismissed_version == state.latest_version && now < state.dismissed_until_ms {
        return None;
    }
    Some(state.latest_version.clone())
}

/// The newer version to point the user at, or `None` when they are up to date or have hidden it.
///
/// A failed request deliberately doesn't count as a check, so a moment without a connection costs
/// the next start's check rather than the whole day's.
pub async fn check<E: Env>(current: &str) -> Option<String> {
    let mut state = Store::<E>::get_update_state().ok().flatten().unwrap_or_default();
    let now = E::default().now_ms();

    if now >= state.last_checked_ms.saturating_add(DAY_MS) {
        if let Some(latest) = fetch_latest::<E>().await {
            state.latest_version = latest;
            state.last_checked_ms = now;
            let _ = Store::<E>::save_update_state(&state);
        }
    }

    available(&state, current, now)
}

/// Hides the banner for the next ten days, or until something newer than `version` comes out
pub fn dismiss<E: Env>(version: &str) {
    let mut state = Store::<E>::get_update_state().ok().flatten().unwrap_or_default();
    state.dismissed_version = version.to_string();
    state.dismissed_until_ms = E::default().now_ms().saturating_add(DISMISS_DAYS * DAY_MS);
    let _ = Store::<E>::save_update_state(&state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::HttpResponse;
    use futures::executor::block_on;
    use std::cell::{Cell, RefCell};

    /// The thread locals below are per test thread, but a single-threaded run would otherwise
    /// hand one test's leftovers to the next
    fn reset() {
        STORE.with(|store| store.borrow_mut().clear());
        REQUESTS.with(|count| count.set(0));
    }

    fn seen(latest: &str) -> UpdateState {
        UpdateState { latest_version: latest.to_string(), ..Default::default() }
    }

    #[test]
    fn compares_versions_by_number_not_by_text() {
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(is_newer("0.2.1", "0.2"));
        assert!(!is_newer("0.2.0", "0.2"));
        assert!(!is_newer("0.2", "0.2.1"));
    }

    /// the published tags are bare, but a `v` prefix shouldn't start a wrong-way comparison
    #[test]
    fn accepts_the_tags_that_are_actually_published() {
        assert!(is_newer("v0.3.0", "0.2.1"));
        assert!(!is_newer("nightly", "0.2.1"));
        assert!(!is_newer("0.3.0", "not a version"));
        assert!(!is_newer("0.1.2.3", "0.1.0"));
    }

    #[test]
    fn shows_nothing_when_up_to_date() {
        assert_eq!(available(&seen("0.2.1"), "0.2.1", 0), None);
        assert_eq!(available(&seen("0.2.0"), "0.2.1", 0), None);
        assert_eq!(available(&UpdateState::default(), "0.2.1", 0), None);
    }

    #[test]
    fn hides_a_dismissed_release_for_ten_days_and_no_longer() {
        let state = UpdateState {
            latest_version: "0.3.0".to_string(),
            dismissed_version: "0.3.0".to_string(),
            dismissed_until_ms: 10 * DAY_MS,
            last_checked_ms: 0,
        };

        assert_eq!(available(&state, "0.2.1", 0), None);
        assert_eq!(available(&state, "0.2.1", 10 * DAY_MS - 1), None);
        assert_eq!(available(&state, "0.2.1", 10 * DAY_MS), Some("0.3.0".to_string()));
    }

    /// hiding one release must not hide the next one behind it
    #[test]
    fn shows_a_release_newer_than_the_dismissed_one() {
        let state = UpdateState {
            latest_version: "0.4.0".to_string(),
            dismissed_version: "0.3.0".to_string(),
            dismissed_until_ms: 10 * DAY_MS,
            last_checked_ms: 0,
        };

        assert_eq!(available(&state, "0.2.1", 0), Some("0.4.0".to_string()));
    }

    /// A stand-in for the browser: the store is a map, the clock is whatever the test says, and
    /// the request hands back a canned release without going near the network.
    #[derive(Default)]
    struct MockEnv;

    thread_local! {
        static STORE: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
        static NOW: Cell<u64> = const { Cell::new(0) };
        static REQUESTS: Cell<u32> = const { Cell::new(0) };
        static BODY: RefCell<String> = const { RefCell::new(String::new()) };
    }

    impl Env for MockEnv {
        fn get(&self, key: &str) -> Option<String> {
            STORE.with(|store| store.borrow().get(key).cloned())
        }

        fn set(&self, key: &str, value: &str) -> Result<(), String> {
            STORE.with(|store| store.borrow_mut().insert(key.to_string(), value.to_string()));
            Ok(())
        }

        fn remove(&self, key: &str) {
            STORE.with(|store| store.borrow_mut().remove(key));
        }

        fn now_ms(&self) -> u64 {
            NOW.with(Cell::get)
        }

        async fn request(
            &self,
            _method: &str,
            _url: &str,
            _headers: HashMap<String, Vec<String>>,
            _body: String,
        ) -> Result<HttpResponse, String> {
            REQUESTS.with(|count| count.set(count.get() + 1));
            Ok(HttpResponse { headers: HashMap::new(), body: BODY.with(|body| body.borrow().clone()) })
        }

        async fn notify(&self, _title: &str, _body: &str) {}
    }

    /// The whole path a start actually takes: ask GitHub, parse the real shape of its answer,
    /// store it, and answer from the store until the day is up.
    #[test]
    fn asks_github_once_a_day_and_remembers_the_answer() {
        reset();
        // trimmed to the fields the check reads, but the shape the API really returns
        BODY.with(|body| {
            *body.borrow_mut() =
                r#"{"tag_name":"0.3.0","name":"0.3.0","draft":false,"prerelease":false}"#.to_string()
        });
        NOW.with(|now| now.set(DAY_MS));

        assert_eq!(block_on(check::<MockEnv>("0.2.1")), Some("0.3.0".to_string()));
        assert_eq!(REQUESTS.with(Cell::get), 1);

        // an hour later the answer comes from the store, not from GitHub
        NOW.with(|now| now.set(DAY_MS + 60 * 60 * 1000));
        assert_eq!(block_on(check::<MockEnv>("0.2.1")), Some("0.3.0".to_string()));
        assert_eq!(REQUESTS.with(Cell::get), 1);

        // hiding it survives a restart
        dismiss::<MockEnv>("0.3.0");
        assert_eq!(block_on(check::<MockEnv>("0.2.1")), None);

        // ten days later it is back
        NOW.with(|now| now.set(DAY_MS + 60 * 60 * 1000 + 10 * DAY_MS));
        assert_eq!(block_on(check::<MockEnv>("0.2.1")), Some("0.3.0".to_string()));
    }

    /// Running the newest release means no banner, however often the app is started
    #[test]
    fn stays_quiet_when_the_latest_release_is_what_is_running() {
        reset();
        BODY.with(|body| *body.borrow_mut() = r#"{"tag_name":"0.2.1"}"#.to_string());
        NOW.with(|now| now.set(DAY_MS));

        assert_eq!(block_on(check::<MockEnv>("0.2.1")), None);
    }
}
