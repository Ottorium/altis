//! The background poller's copy of the settings and its own bookkeeping.
//!
//! The frontend keeps all of this in the webview's localStorage, which the poller can't reach
//! once the app is closed, so it lands in files here instead. There are two of them, because the
//! service runs in its own process (see the manifest for why) and two processes writing one file
//! would race:
//!
//! - `synced_from_app.json` is written only by the app (the `sync_store` command) and only read
//!   by the poller. It holds `SHARED_KEYS`, i.e. the settings.
//! - `poller_state.json` is written and read only by the poller: its own Untis session and the
//!   notification bookkeeping.
//!
//! Neither file is ever written by both processes, so no locking is needed. The synced one is
//! re-read on every access, since the process that writes it is not this one.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const SYNCED_FILE: &str = "synced_from_app.json";
const OWNED_FILE: &str = "poller_state.json";

struct Paths {
    synced: PathBuf,
    owned: PathBuf,
}

static PATHS: OnceLock<Paths> = OnceLock::new();
static OWNED: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

/// Points the store at the app's private files directory, which both processes share. Safe to
/// call more than once: the activity and the service each do it for their own process.
pub fn init(dir: &Path) {
    let paths = PATHS.get_or_init(|| Paths {
        synced: dir.join(SYNCED_FILE),
        owned: dir.join(OWNED_FILE),
    });

    OWNED.get_or_init(|| Mutex::new(read_map(&paths.owned)));
}

fn read_map(path: &Path) -> HashMap<String, String> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Writes through a temporary file, so a reader in the other process sees either the old contents
/// or the new ones, never half of each — and a process that dies mid-write (on Android, an
/// entirely ordinary way for one to end) leaves the previous state intact
fn write_map(path: &Path, map: &HashMap<String, String>) -> Result<(), String> {
    let serialized = serde_json::to_string(map).map_err(|e| e.to_string())?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, serialized).map_err(|e| e.to_string())?;
    fs::rename(&temp, path).map_err(|e| e.to_string())
}

/// Whether the app owns this key, in which case it only ever arrives through [`sync`]
fn is_synced(key: &str) -> bool {
    altis_core::store::SHARED_KEYS.contains(&key)
}

fn owned<R>(f: impl FnOnce(&mut HashMap<String, String>) -> R) -> Result<R, String> {
    let mut guard = OWNED
        .get()
        .ok_or("The background store is not initialized")?
        .lock()
        .map_err(|_| "The background store is poisoned".to_string())?;
    Ok(f(&mut guard))
}

pub fn get(key: &str) -> Option<String> {
    if is_synced(key) {
        return read_map(&PATHS.get()?.synced).remove(key);
    }
    owned(|map| map.get(key).cloned()).ok().flatten()
}

pub fn set(key: &str, value: &str) -> Result<(), String> {
    if is_synced(key) {
        // would race the app, which is the only writer of the synced file. Nothing in the poller
        // does this - it reads the settings and writes only its own state - so if this ever fires
        // it is a bug worth seeing rather than a write worth guessing at.
        return Err(format!("{key} belongs to the app and cannot be written by the poller"));
    }

    let path = &PATHS.get().ok_or("The background store is not initialized")?.owned;
    owned(|map| {
        map.insert(key.to_string(), value.to_string());
        write_map(path, map)
    })?
}

pub fn remove(key: &str) {
    if is_synced(key) {
        return;
    }
    let Some(paths) = PATHS.get() else { return };

    let _ = owned(|map| {
        if map.remove(key).is_some() {
            let _ = write_map(&paths.owned, map);
        }
    });
}

/// Replaces the synced keys with the app's copies. Called in the app's process, not the poller's.
/// Keys the app no longer has (a login that was cleared, say) are dropped rather than left behind
/// for the next poll to use.
pub fn sync(keys: &[&str], entries: HashMap<String, String>) -> Result<(), String> {
    let path = &PATHS.get().ok_or("The background store is not initialized")?.synced;

    let mut map = read_map(path);
    for key in keys {
        match entries.get(*key) {
            Some(value) => { map.insert((*key).to_string(), value.clone()); }
            None => { map.remove(*key); }
        }
    }
    write_map(path, &map)
}
