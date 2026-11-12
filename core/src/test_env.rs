//! A stand-in for the platform in tests: the store is a map, the clock is whatever the test says,
//! and requests are answered by the test without going near the network.

use crate::env::{Env, Headers, HttpResponse};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// A request as the code under test sent it
#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: Headers,
    pub body: String,
}

type Responder = Box<dyn Fn(&Request) -> Result<HttpResponse, String>>;

thread_local! {
    static STORE: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    static NOW_MS: Cell<u64> = const { Cell::new(0) };
    static REQUESTS: RefCell<Vec<Request>> = const { RefCell::new(Vec::new()) };
    static NOTIFICATIONS: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    static RESPONDER: RefCell<Option<Responder>> = const { RefCell::new(None) };
}

#[derive(Default)]
pub struct TestEnv;

impl TestEnv {
    /// Tests share a thread when run single-threaded, so each starts by clearing what the last
    /// one left behind
    pub fn reset() {
        STORE.with(|store| store.borrow_mut().clear());
        NOW_MS.with(|now| now.set(0));
        REQUESTS.with(|requests| requests.borrow_mut().clear());
        NOTIFICATIONS.with(|notifications| notifications.borrow_mut().clear());
        RESPONDER.with(|responder| *responder.borrow_mut() = None);
    }

    pub fn set_now_ms(now: u64) {
        NOW_MS.with(|clock| clock.set(now));
    }

    /// Decides what every request from here on is answered with
    pub fn respond_with(responder: impl Fn(&Request) -> Result<HttpResponse, String> + 'static) {
        RESPONDER.with(|slot| *slot.borrow_mut() = Some(Box::new(responder)));
    }

    /// Answers every request from here on with the same body
    pub fn respond_with_body(body: &str) {
        let body = body.to_string();
        Self::respond_with(move |_| Ok(HttpResponse { headers: Headers::new(), body: body.clone() }));
    }

    pub fn requests() -> Vec<Request> {
        REQUESTS.with(|requests| requests.borrow().clone())
    }

    /// The notifications shown so far, as (title, body), handing them over so the next look only
    /// sees what came after
    pub fn take_notifications() -> Vec<(String, String)> {
        NOTIFICATIONS.with(|notifications| notifications.take())
    }
}

impl Env for TestEnv {
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
        NOW_MS.with(Cell::get)
    }

    async fn request(&self, method: &str, url: &str, headers: Headers, body: String) -> Result<HttpResponse, String> {
        let request = Request { method: method.to_string(), url: url.to_string(), headers, body };
        REQUESTS.with(|requests| requests.borrow_mut().push(request.clone()));

        RESPONDER.with(|responder| match &*responder.borrow() {
            Some(respond) => respond(&request),
            None => Err(format!("No response set up for {url}")),
        })
    }

    async fn notify(&self, title: &str, body: &str) {
        NOTIFICATIONS.with(|notifications| notifications.borrow_mut().push((title.to_string(), body.to_string())));
    }
}
