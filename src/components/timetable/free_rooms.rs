//! Which rooms no lesson takes up, period by period - the timetables of every room turned inside out.

use super::day_heading;
use crate::components::now_line::NowLine;
use crate::hooks::use_now;
use altis_core::models::timetable::{Entity, Room, WeekTimeTable};
use altis_core::settings::WeekdayOverride;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};
use std::collections::HashMap;
use yew::prelude::*;

/// A time a room is taken, as (date, start, end)
type Booking = (NaiveDate, NaiveTime, NaiveTime);

/// A room of the school along with every lesson it is booked for that week
struct RoomBookings {
    name: String,
    bookings: Vec<Booking>,
}

/// One row of the grid: a stretch of time no lesson starts or ends inside, and the rooms free
/// during it on each of the shown days
#[derive(Clone, Debug, PartialEq)]
struct Period {
    start: NaiveTime,
    end: NaiveTime,
    /// the free rooms per day, in the order of the days the grid was built for
    free: Vec<Vec<String>>,
}

/// Every room the week's timetables mention, sorted by name. A cancelled lesson doesn't book its
/// room - that is the whole point of one for this view
fn rooms(all: &HashMap<Entity, WeekTimeTable>) -> Vec<RoomBookings> {
    let mut rooms: Vec<RoomBookings> = all
        .iter()
        .filter_map(|(entity, table)| match entity {
            Entity::Room(room) if !room.name.is_empty() => Some(RoomBookings {
                name: room.name.clone(),
                bookings: table
                    .days
                    .iter()
                    .flat_map(|day| {
                        day.lessons
                            .iter()
                            .filter(|lesson| !lesson.is_cancelled())
                            .map(move |lesson| {
                                (day.date, lesson.time_range.start.time(), lesson.time_range.end.time())
                            })
                    })
                    .collect(),
            }),
            _ => None,
        })
        .collect();
    rooms.sort_by(|a, b| a.name.cmp(&b.name));
    rooms
}

/// The days the week's timetables cover, in order
fn week_days(all: &HashMap<Entity, WeekTimeTable>) -> Vec<NaiveDate> {
    let mut days: Vec<NaiveDate> = all
        .values()
        .flat_map(|table| table.days.iter().map(|day| day.date))
        .collect();
    days.sort();
    days.dedup();
    days
}

/// Splits the week at every time a lesson starts or ends, keeping the stretches some lesson covers.
/// A double lesson therefore blocks its room for each of the single periods it spans
fn periods(rooms: &[RoomBookings], days: &[NaiveDate]) -> Vec<(NaiveTime, NaiveTime)> {
    let shown = |(date, _, _): &&Booking| days.contains(date);
    let bookings = || rooms.iter().flat_map(|room| room.bookings.iter()).filter(shown);

    let mut bounds: Vec<NaiveTime> = bookings().flat_map(|&(_, start, end)| [start, end]).collect();
    bounds.sort();
    bounds.dedup();

    bounds
        .windows(2)
        .map(|w| (w[0], w[1]))
        .filter(|&(start, end)| bookings().any(|&(_, s, e)| s <= start && end <= e))
        .collect()
}

/// The rooms not in use, for every period of the week and every day it is shown for
fn free_rooms(rooms: &[RoomBookings], days: &[NaiveDate]) -> Vec<Period> {
    periods(rooms, days)
        .into_iter()
        .map(|(start, end)| Period {
            start,
            end,
            free: days
                .iter()
                .map(|&date| {
                    rooms
                        .iter()
                        .filter(|room| {
                            !room.bookings.iter().any(|&(d, s, e)| d == date && s < end && start < e)
                        })
                        .map(|room| room.name.clone())
                        .collect()
                })
                .collect(),
        })
        .collect()
}

/// The grid of a week: which rooms are free in which period, ready to render
#[derive(Clone, Debug, PartialEq)]
pub struct FreeRooms {
    /// the days of the week that are shown, in order
    days: Vec<NaiveDate>,
    periods: Vec<Period>,
}

impl FreeRooms {
    /// Works out the free rooms of a week from the timetables of everything in it
    pub fn of(all: &HashMap<Entity, WeekTimeTable>, weekday_override: &WeekdayOverride) -> Self {
        let rooms = rooms(all);
        let days: Vec<NaiveDate> = week_days(all).into_iter()
            .filter(|date| {
                let has_lessons = rooms.iter().any(|room| room.bookings.iter().any(|(d, _, _)| d == date));
                weekday_override.should_show(date.weekday(), has_lessons)
            })
            .collect();

        let periods = free_rooms(&rooms, &days);
        Self { days, periods }
    }
}

#[derive(Properties, PartialEq)]
pub struct FreeRoomsProps {
    pub free_rooms: FreeRooms,
    /// whether to mark the current time
    pub show_now_line: bool,
    /// a room was clicked, to show its timetable
    pub on_entity_select: Callback<Entity>,
}

#[function_component(FreeRoomsView)]
pub fn free_rooms_view(props: &FreeRoomsProps) -> Html {
    let FreeRooms { days, periods } = &props.free_rooms;
    let now = use_now();

    if periods.is_empty() {
        return html! { <div class="p-3">{ "No lessons!" }</div> };
    }

    // the grid lives inside the week swipe, so it has to fit the screen - a sideways scroll in
    // there would fight the swipe (which is why `.week-track *` only pans vertically)
    let columns = format!("grid-template-columns: 3.2rem repeat({}, minmax(0, 1fr));", days.len());

    html! {
        <div class="flex-grow-1 overflow-auto p-3">
            <div class="free-rooms-grid" style={columns}>
                <div class="free-rooms-head free-rooms-time"></div>
                { for days.iter().map(|date| html! {
                    <div class="free-rooms-head text-center">{ day_heading(*date) }</div>
                })}

                { for periods.iter().map(|period| html! {
                    <>
                        <div class="free-rooms-time small text-secondary">
                            <div>{ period.start.format("%H:%M").to_string() }</div>
                            <div>{ period.end.format("%H:%M").to_string() }</div>
                        </div>
                        { for days.iter().zip(&period.free).map(|(date, free)| html! {
                            <div class="free-rooms-cell">
                                { if free.is_empty() {
                                    html! { <span class="text-secondary small">{ "none free" }</span> }
                                } else {
                                    free.iter()
                                        .map(|name| render_room(name, &props.on_entity_select))
                                        .collect::<Html>()
                                }}
                                if props.show_now_line {
                                    { render_now_line(period, *date, now) }
                                }
                            </div>
                        })}
                    </>
                })}
            </div>
        </div>
    }
}

/// The line marking the current time, the same one the timetable draws. A row is as tall as its
/// content rather than its duration, so the line sits where `now` falls inside the period
fn render_now_line(period: &Period, date: NaiveDate, now: NaiveDateTime) -> Html {
    if date != now.date() || !(period.start..period.end).contains(&now.time()) {
        return html! {};
    }

    let elapsed = (now.time() - period.start).num_seconds() as f64;
    let duration = (period.end - period.start).num_seconds() as f64;
    let top = elapsed / duration * 100.0;

    html! { <NowLine {top} /> }
}

/// A free room, which opens its own timetable when clicked
fn render_room(name: &str, on_entity_select: &Callback<Entity>) -> Html {
    let onclick = {
        let cb = on_entity_select.clone();
        let room = Entity::Room(Room { name: name.to_string() });
        Callback::from(move |_| cb.emit(room.clone()))
    };

    html! {
        <button type="button" class="badge border-0 free-room-badge" {onclick}>{ name }</button>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altis_core::models::timetable::{DayTimeTable, LessonBlock, LessonStatus, TimeRange};

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, day).unwrap()
    }

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn lesson(day: u32, start: (u32, u32), end: (u32, u32)) -> LessonBlock {
        LessonBlock {
            time_range: TimeRange {
                start: date(day).and_time(t(start.0, start.1)),
                end: date(day).and_time(t(end.0, end.1)),
            },
            ..Default::default()
        }
    }

    fn cancelled(mut lesson: LessonBlock) -> LessonBlock {
        lesson.status = LessonStatus::Cancelled;
        lesson
    }

    /// the timetables of the given rooms, over the Monday and Tuesday the lessons are on
    fn week(rooms: &[(&str, Vec<LessonBlock>)]) -> HashMap<Entity, WeekTimeTable> {
        rooms.iter()
            .map(|(name, lessons)| {
                let days = [14, 15].into_iter()
                    .map(|day| DayTimeTable {
                        date: date(day),
                        lessons: lessons.iter().filter(|l| l.time_range.start.date() == date(day)).cloned().collect(),
                    })
                    .collect();
                (Entity::Room(Room { name: name.to_string() }), WeekTimeTable { days })
            })
            .collect()
    }

    fn free(all: &HashMap<Entity, WeekTimeTable>) -> Vec<Period> {
        free_rooms(&rooms(all), &[date(14), date(15)])
    }

    #[test]
    fn a_room_is_free_where_it_has_no_lesson() {
        let all = week(&[
            ("A1", vec![lesson(14, (8, 0), (8, 45))]),
            ("A2", vec![]),
        ]);

        let periods = free(&all);
        assert_eq!(periods.len(), 1);
        assert_eq!(periods[0].start, t(8, 0));
        assert_eq!(periods[0].end, t(8, 45));
        // Monday has the lesson, Tuesday doesn't
        assert_eq!(periods[0].free, vec![vec!["A2".to_string()], vec!["A1".to_string(), "A2".to_string()]]);
    }

    #[test]
    fn a_cancelled_lesson_frees_the_room() {
        let all = week(&[
            ("A1", vec![cancelled(lesson(14, (8, 0), (8, 45)))]),
            ("A2", vec![lesson(14, (8, 0), (8, 45))]),
        ]);

        assert_eq!(free(&all)[0].free[0], vec!["A1".to_string()]);
    }

    #[test]
    fn a_double_lesson_blocks_every_period_it_spans() {
        let all = week(&[
            ("A1", vec![lesson(14, (8, 0), (9, 35))]),
            ("A2", vec![lesson(14, (8, 0), (8, 45)), lesson(14, (8, 50), (9, 35))]),
        ]);

        let periods = free(&all);
        assert_eq!(
            periods.iter().map(|p| (p.start, p.end)).collect::<Vec<_>>(),
            vec![(t(8, 0), t(8, 45)), (t(8, 45), t(8, 50)), (t(8, 50), t(9, 35))],
        );
        // the room taken by the double lesson is free in none of them, not even over the break
        assert!(periods.iter().all(|p| !p.free[0].contains(&"A1".to_string())));
        // the break between the two single lessons is the only time the other one is free
        assert_eq!(periods.iter().map(|p| p.free[0].clone()).collect::<Vec<_>>(), vec![
            Vec::<String>::new(),
            vec!["A2".to_string()],
            Vec::<String>::new(),
        ]);
    }

    #[test]
    fn free_periods_of_the_day_are_no_period() {
        let all = week(&[
            ("A1", vec![lesson(14, (8, 0), (8, 45)), lesson(14, (13, 0), (13, 45))]),
        ]);

        assert_eq!(
            free(&all).iter().map(|p| (p.start, p.end)).collect::<Vec<_>>(),
            vec![(t(8, 0), t(8, 45)), (t(13, 0), t(13, 45))],
        );
    }

    #[test]
    fn a_day_that_is_not_shown_is_left_out() {
        let all = week(&[("A1", vec![lesson(14, (8, 0), (8, 45))])]);

        let periods = free_rooms(&rooms(&all), &[date(14)]);
        assert_eq!(periods.len(), 1);
        assert_eq!(periods[0].free, vec![Vec::<String>::new()]);
    }
}
