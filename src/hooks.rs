use chrono::{Local, NaiveDateTime};
use gloo_timers::callback::Interval;
use yew::prelude::*;

/// How often something drawn by the current time is brought up to date
const NOW_REFRESH_MS: u32 = 30_000;

fn current_time() -> NaiveDateTime {
    Local::now().naive_local()
}

/// The current time, re-rendering the component every half minute so what is drawn by it - the
/// line marking now, the lessons that are over - moves along
#[hook]
pub fn use_now() -> NaiveDateTime {
    let now = use_state(current_time);

    {
        let now = now.clone();
        use_effect_with((), move |_| {
            let interval = Interval::new(NOW_REFRESH_MS, move || now.set(current_time()));
            move || drop(interval)
        });
    }

    *now
}
