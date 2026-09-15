//! The JNI edge between the Kotlin foreground service and the Rust poller.
//!
//! Kotlin calls in to set the poller up and start or stop it; Rust calls back out to post a
//! notification, because the notification plugin talks to the activity, which is exactly what
//! isn't there any more once the app has been closed.

use jni::objects::{GlobalRef, JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static VM: OnceLock<JavaVM> = OnceLock::new();
/// A global ref to the `com.altis.app.BackgroundPoller` singleton. Kept rather than looked up on
/// demand: a thread attached from native code gets the system class loader, which knows nothing
/// about the app's own classes, so by the time the poller wants to notify it is far too late to
/// go looking for them.
static POLLER: OnceLock<GlobalRef> = OnceLock::new();

static SHOULD_RUN: AtomicBool = AtomicBool::new(false);
static THREAD_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn should_run() -> bool {
    SHOULD_RUN.load(Ordering::SeqCst)
}

#[no_mangle]
pub extern "system" fn Java_com_altis_app_BackgroundPoller_nativeInit(
    mut env: JNIEnv,
    poller: JObject,
    files_dir: JString,
) {
    // the service can be what starts the process, in which case `run()` never got the chance to
    crate::http::install_crypto_provider();

    if let Ok(vm) = env.get_java_vm() {
        let _ = VM.set(vm);
    }
    if let Ok(global) = env.new_global_ref(&poller) {
        let _ = POLLER.set(global);
    }
    if let Ok(dir) = env.get_string(&files_dir) {
        let dir: String = dir.into();
        super::store::init(Path::new(&dir));
    }
}

#[no_mangle]
pub extern "system" fn Java_com_altis_app_BackgroundPoller_nativeStartPolling(_env: JNIEnv, _poller: JObject) {
    SHOULD_RUN.store(true, Ordering::SeqCst);

    if THREAD_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    let spawned = std::thread::Builder::new()
        .name("altis-poller".to_string())
        .spawn(|| {
            super::poll_loop();
            THREAD_RUNNING.store(false, Ordering::SeqCst);
        });

    if spawned.is_err() {
        THREAD_RUNNING.store(false, Ordering::SeqCst);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_altis_app_BackgroundPoller_nativeStopPolling(_env: JNIEnv, _poller: JObject) {
    SHOULD_RUN.store(false, Ordering::SeqCst);
}

/// One poll, on the calling thread. This is what the WorkManager job runs: it has no loop of its
/// own to keep, the schedule is Android's.
#[no_mangle]
pub extern "system" fn Java_com_altis_app_BackgroundPoller_nativeRunOnce(_env: JNIEnv, _poller: JObject) {
    super::run_once_blocking();
}

fn with_poller<R>(
    call: impl FnOnce(&mut JNIEnv, &JObject) -> Result<R, jni::errors::Error>,
) -> Result<R, String> {
    let vm = VM.get().ok_or("The Android background bridge is not initialized")?;
    let poller = POLLER.get().ok_or("The Android background bridge is not initialized")?;

    let mut guard = vm.attach_current_thread().map_err(|e| e.to_string())?;
    let env = &mut *guard;

    call(env, poller.as_obj()).map_err(|e| {
        // a pending exception poisons every later call on this thread
        let _ = env.exception_clear();
        e.to_string()
    })
}

/// Shows a notification through the service, which holds the application context and the channel
pub fn post_notification(title: &str, body: &str) -> Result<(), String> {
    with_poller(|env, poller| {
        let title = env.new_string(title)?;
        let body = env.new_string(body)?;
        env.call_method(
            poller,
            "postNotification",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            &[JValue::Object(&title), JValue::Object(&body)],
        )?;
        Ok(())
    })
}

/// Switches between the WorkManager job, the foreground service and neither, and remembers the
/// choice for the next boot
pub fn set_background_mode(mode: &str) -> Result<(), String> {
    with_poller(|env, poller| {
        let mode = env.new_string(mode)?;
        env.call_method(poller, "setBackgroundMode", "(Ljava/lang/String;)V", &[JValue::Object(&mode)])?;
        Ok(())
    })
}
