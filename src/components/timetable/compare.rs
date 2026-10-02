use crate::persistence_manager::PersistenceManager;
use altis_core::data_models::clean_models::untis::{ChangeStatus, Entity, LessonBlock, WeekTimeTable};
use altis_core::settings::{Favorites, is_dark_color};
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use gloo_timers::callback::Interval;
use serde::{Deserialize, Serialize};
use web_sys::{HtmlInputElement, MouseEvent};
use yew::prelude::*;

/// A timetable picked for comparing: a category of the timetable picker and the name it shows
/// there, or the category "Me" for the personal timetable
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Compared {
    pub category: String,
    pub name: String,
}

impl Compared {
    pub fn me() -> Self {
        Self { category: "Me".to_string(), name: String::new() }
    }

    pub fn is_me(&self) -> bool {
        self.category == "Me"
    }

    pub fn label(&self) -> String {
        if self.is_me() { "Me".to_string() } else { self.name.clone() }
    }
}

fn category_icon(category: &str) -> &'static str {
    match category {
        "Class" => "bi-people",
        "Teacher" => "bi-person-badge",
        "Room" => "bi-geo-alt",
        _ => "bi-person",
    }
}

/// A time span on one day, as (start, end)
type Span = (NaiveTime, NaiveTime);

/// One day of the comparison: every compared timetable's lessons
#[derive(Clone, PartialEq, Debug)]
struct Day {
    date: NaiveDate,
    /// the lessons per compared timetable, in the order of the comparison
    lanes: Vec<Vec<LessonBlock>>,
}

/// A week of several timetables side by side, ready to render
#[derive(Clone, PartialEq, Debug)]
pub struct Comparison {
    /// the compared timetables, in the order of the lanes
    names: Vec<String>,
    /// compared timetables that aren't in this week
    missing: Vec<String>,
    days: Vec<Day>,
    start: NaiveTime,
    end: NaiveTime,
}

impl Comparison {
    pub fn of(tables: &[(String, &WeekTimeTable)], missing: Vec<String>) -> Self {
        let visual_settings = PersistenceManager::get_settings().ok().flatten()
            .map(|settings| settings.visual_settings).unwrap_or_default();

        let mut dates: Vec<NaiveDate> = tables.iter()
            .flat_map(|(_, table)| table.days.iter().map(|day| day.date))
            .collect();
        dates.sort();
        dates.dedup();

        let days: Vec<Day> = dates.into_iter()
            .map(|date| {
                let lanes: Vec<Vec<LessonBlock>> = tables.iter()
                    .map(|(_, table)| {
                        let mut lessons: Vec<LessonBlock> = table.days.iter()
                            .filter(|day| day.date == date)
                            .flat_map(|day| day.lessons.iter().cloned())
                            .collect();
                        for lesson in &mut lessons {
                            if let Some(color) = visual_settings.get_lesson_color_override(lesson) {
                                lesson.color_hex = color.trim_start_matches('#').to_string();
                            }
                        }
                        lessons
                    })
                    .collect();
                Day { date, lanes }
            })
            .filter(|day| {
                let has_lessons = day.lanes.iter().any(|lessons| !lessons.is_empty());
                visual_settings.weekday_override.should_show(day.date.weekday(), has_lessons)
            })
            .collect();

        let lessons = || days.iter().flat_map(|day| day.lanes.iter().flatten());
        let (start, end) = match (lessons().map(|l| l.time_range.start.time()).min(), lessons().map(|l| l.time_range.end.time()).max()) {
            (Some(start), Some(end)) => visual_settings.padded_range(start, end),
            _ => (NaiveTime::MIN, NaiveTime::MIN),
        };

        Self {
            names: tables.iter().map(|(name, _)| name.clone()).collect(),
            missing,
            days,
            start,
            end,
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct CompareRenderProps {
    pub comparison: Comparison,
}

#[function_component(CompareRender)]
pub fn compare_render(props: &CompareRenderProps) -> Html {
    let Comparison { names, missing, days, start, end } = &props.comparison;
    let now = use_state(current_time);

    {
        let now = now.clone();
        use_effect_with((), move |_| {
            let interval = Interval::new(30_000, move || now.set(current_time()));
            move || drop(interval)
        });
    }

    let missing_note = (!missing.is_empty()).then(|| html! {
        <div class="small text-secondary mb-2">
            <i class="bi bi-exclamation-circle me-1"></i>
            { format!("Not in this week: {}", missing.join(", ")) }
        </div>
    });

    let total = (*end - *start).num_seconds() as f64;
    if days.iter().all(|day| day.lanes.iter().all(Vec::is_empty)) || total <= 0.0 {
        return html! { <div class="p-3">{ missing_note }{ "No lessons!" }</div> };
    }
    let percent = |time: NaiveTime| (time - *start).num_seconds() as f64 / total * 100.0;
    let span_style = |(from, until): Span| format!("top: {}%; height: {}%;", percent(from), percent(until) - percent(from));

    let hours: Vec<NaiveTime> = (start.hour()..=end.hour())
        .filter_map(|h| NaiveTime::from_hms_opt(h, 0, 0))
        .filter(|t| start <= t && t <= end)
        .collect();

    let legend = names.iter().enumerate().map(|(i, name)| html! {
        <span class="badge compare-legend">
            <span class="compare-lane-number">{ i + 1 }</span>{ name }
        </span>
    }).collect::<Html>();

    let render_day = |day: &Day| {
        let now_top = (day.date == now.date() && (*start..=*end).contains(&now.time())).then(|| percent(now.time()));
        html! {
            <div class="compare-day">
                { for hours.iter().map(|&hour| html! {
                    <div class="compare-hour-line" style={format!("top: {}%;", percent(hour))}></div>
                })}
                <div class="d-flex h-100">
                    { for day.lanes.iter().map(|lessons| html! {
                        <div class="compare-lane">
                            { for lessons.iter().map(|lesson| render_lesson(lesson, span_style((lesson.time_range.start.time(), lesson.time_range.end.time())), *now)) }
                        </div>
                    })}
                </div>
                if let Some(top) = now_top {
                    <div class="compare-now-line" style={format!("top: {top}%;")}></div>
                }
            </div>
        }
    };

    html! {
        <div class="d-flex flex-column p-2 gap-2" style="min-height: 100%;">
            <div class="d-flex flex-wrap align-items-center gap-1">{ legend }</div>
            { missing_note }

            <div class="d-flex flex-column flex-grow-1" style="min-height: 24rem;">
                <div class="d-flex border-bottom">
                    <div class="compare-time-column"></div>
                    { for days.iter().map(|day| html! {
                        <div class="compare-day-head text-center pb-1">
                            <div class="fw-bold">{ day.date.weekday().to_string() }</div>
                            <div class="small">{ day.date.format("%d.%m").to_string() }</div>
                            <div class="d-flex compare-lane-numbers">
                                { for (1..=day.lanes.len()).map(|i| html! { <span>{ i }</span> }) }
                            </div>
                        </div>
                    })}
                </div>
                <div class="d-flex flex-grow-1">
                    <div class="compare-time-column position-relative">
                        // the label sits below its line, so the last hour's would hang out of the grid
                        { for hours.iter().filter(|&hour| hour < end).map(|&hour| html! {
                            <div class="compare-hour-label small text-secondary" style={format!("top: {}%;", percent(hour))}>
                                { hour.format("%H:%M").to_string() }
                            </div>
                        })}
                    </div>
                    { for days.iter().map(render_day) }
                </div>
            </div>
        </div>
    }
}

fn current_time() -> NaiveDateTime {
    Local::now().naive_local()
}

fn format_span((from, until): Span) -> String {
    format!("{}-{}", from.format("%H:%M"), until.format("%H:%M"))
}

fn render_lesson(lesson: &LessonBlock, position: String, now: NaiveDateTime) -> Html {
    let first_name = |wanted: fn(&Entity) -> bool| lesson.entities.iter()
        .filter(|e| e.status != ChangeStatus::Removed && wanted(&e.inner))
        .map(|e| e.inner.name())
        .find(|name| !name.is_empty());
    let subject = first_name(|e| matches!(e, Entity::Subject(_))).unwrap_or_else(|| lesson.r#type.clone());
    let room = first_name(|e| matches!(e, Entity::Room(_)));

    let color = lesson.color_hex.trim_start_matches('#');
    let text_cls = if is_dark_color(color) { "text-white" } else { "text-black" };
    let status_cls = match lesson.status.as_str() {
        "CANCELLED" => Some("compare-lesson-cancelled"),
        "CHANGED" => Some("compare-lesson-changed"),
        "ADDITIONAL" => Some("compare-lesson-additional"),
        _ if lesson.r#type == "EXAM" => Some("compare-lesson-exam"),
        _ => None,
    };
    let past = lesson.time_range.end <= now;
    let title = format!("{subject} {}", format_span((lesson.time_range.start.time(), lesson.time_range.end.time())));

    html! {
        <div class={classes!("compare-lesson", past.then_some("compare-lesson-past"))} style={position} {title}>
            <div class={classes!("compare-lesson-inner", text_cls, status_cls)} style={format!("background-color: #{color};")}>
                <span class="fw-semibold">{ subject }</span>
                if let Some(room) = room {
                    <span class="compare-lesson-detail">{ room }</span>
                }
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct ComparePickerProps {
    /// the names to pick from per category, empty while the week is still loading
    pub options: Vec<(&'static str, Vec<String>)>,
    pub favorites: Favorites,
    pub compared: Vec<Compared>,
    pub on_change: Callback<Vec<Compared>>,
    pub on_close: Callback<()>,
}

#[function_component(ComparePicker)]
pub fn compare_picker(props: &ComparePickerProps) -> Html {
    let filter = use_state(String::new);

    let on_filter = {
        let filter = filter.clone();
        Callback::from(move |e: InputEvent| filter.set(e.target_unchecked_into::<HtmlInputElement>().value()))
    };

    let needle = filter.trim().to_lowercase();
    let matches = |name: &str| needle.is_empty() || name.to_lowercase().contains(&needle);

    let chip = |item: Compared| {
        let active = props.compared.contains(&item);
        let onclick = {
            let (compared, on_change, item) = (props.compared.clone(), props.on_change.clone(), item.clone());
            Callback::from(move |_| {
                let mut next = compared.clone();
                match next.iter().position(|c| *c == item) {
                    Some(pos) => { next.remove(pos); }
                    None => next.push(item.clone()),
                }
                on_change.emit(next);
            })
        };
        html! {
            <button type="button" {onclick}
                    class={classes!("btn", "btn-sm", "rounded-pill", if active { "btn-primary" } else { "btn-outline-secondary" })}>
                <i class={classes!("bi", category_icon(&item.category), "me-1")}></i>{ item.label() }
            </button>
        }
    };
    let section = |title: &str, items: Vec<Compared>| {
        if items.is_empty() { return html! {}; }
        html! {
            <div class="mb-3">
                <div class="small fw-bold text-secondary mb-1">{ title.to_string() }</div>
                <div class="d-flex flex-wrap gap-1">{ for items.into_iter().map(chip) }</div>
            </div>
        }
    };
    let item = |category: &str, name: &str| Compared { category: category.to_string(), name: name.to_string() };

    let favorites: Vec<Compared> = [("Class", &props.favorites.classes), ("Teacher", &props.favorites.teachers), ("Room", &props.favorites.rooms)]
        .into_iter()
        .flat_map(|(category, names)| names.iter().filter(|n| matches(n)).map(move |n| item(category, n)))
        .collect();
    let me: Vec<Compared> = matches("me").then(Compared::me).into_iter().collect();

    let on_close = props.on_close.clone();
    html! {
        <div class="modal d-block" style="background: rgba(0,0,0,0.85); z-index: 1050;" onclick={
            let on_close = on_close.clone();
            move |_| on_close.emit(())
        }>
            <div class="modal-dialog modal-lg modal-dialog-centered modal-dialog-scrollable" onclick={|e: MouseEvent| e.stop_propagation()}>
                <div class="modal-content border-primary shadow-lg bg-dark text-light">
                    <div class="modal-header border-primary bg-black text-white">
                        <h5 class="modal-title fw-bold">{ "Compare Timetables" }</h5>
                        <button type="button" class="btn-close btn-close-white" onclick={
                            let on_close = on_close.clone();
                            move |_| on_close.emit(())
                        }></button>
                    </div>
                    <div class="modal-body" style="background-color: #1a1d20;">
                        <div class="small text-secondary mb-2">{ "Pick two or more to show side by side." }</div>
                        <input type="search" class="form-control form-control-sm bg-dark text-light border-secondary mb-3"
                               placeholder="Search" value={(*filter).clone()} oninput={on_filter} />
                        { section("Comparing", props.compared.clone()) }
                        { section("Me", me) }
                        { section("Favourites", favorites) }
                        if props.options.is_empty() {
                            <div class="small text-secondary">{ "Classes, teachers and rooms show up here once the week has loaded." }</div>
                        }
                        { for props.options.iter().map(|(category, names)| {
                            let title = match *category { "Class" => "Classes", "Teacher" => "Teachers", _ => "Rooms" };
                            section(title, names.iter().filter(|n| matches(n)).map(|n| item(category, n)).collect())
                        })}
                    </div>
                    <div class="modal-footer border-primary bg-black">
                        <button type="button" class="btn btn-primary" onclick={move |_| on_close.emit(())}>{ "Done" }</button>
                    </div>
                </div>
            </div>
        </div>
    }
}
