use crate::data_models::clean_models::clean_models::{LessonBlock, WeekTimeTable};
use chrono::{Datelike, Timelike};
use yew::{function_component, html, Html, Properties};

#[derive(Properties, PartialEq, Clone)]
pub struct TimeTableRenderProps {
    pub timetable: WeekTimeTable,
}

#[function_component(TimeTableRender)]
pub fn time_table_render(props: &TimeTableRenderProps) -> Html {
    let lessons: Vec<LessonBlock> = props.timetable.days.iter().flat_map(|dtt| dtt.lessons.clone()).collect();

    if lessons.is_empty() {
        return html! { "No lessons!" };
    }

    let start_time = lessons.iter().map(|l| l.time_range.start).min().unwrap();
    let end_time = lessons.iter().map(|l| l.time_range.end).max().unwrap();

    html! {
        <>
            <div class="d-flex w-100 bg-dark border-bottom">
                <div style="width: 60px;" class="flex-shrink-0"></div>
                <div class="d-flex flex-grow-1">
                    { for props.timetable.days.iter().map(|day| {
                        let weekday = day.date.weekday().to_string();
                        let date_str = day.date.format("%d.%m").to_string();
                        html! {
                            <div class="flex-grow-1 text-center border-start pb-1" style="flex-basis: 0;">
                                <div class="fw-bold">{ weekday }</div>
                                <div class="small">{ date_str }</div>
                            </div>
                        }
                    })}
                </div>
            </div>


            <div class="d-flex flex-grow-1 w-100">
                <div style="width: 60px" class="d-flex flex-column justify-content-between align-items-end">
                    <div class="small pe-1">{ start_time.format("%H:%M").to_string() }</div>
                    <div class="small pe-1">{ end_time.format("%H:%M").to_string() }</div>
                </div>
                <div class="d-flex flex-grow-1">
                    { for props.timetable.days.iter().map(|day| html! {
                        <div class="flex-grow-1 border-start position-relative d-flex">
                            { "Day Content" }
                        </div>
                    })}
                </div>
            </div>
        </>
    }
}
