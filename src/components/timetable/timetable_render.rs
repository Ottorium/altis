use crate::data_models::clean_models::clean_models::WeekTimeTable;
use yew::{function_component, html, Html, Properties};

#[derive(Properties, PartialEq, Clone)]
pub struct TimeTableRenderProps {
    pub timetable: WeekTimeTable,
}

#[function_component(TimeTableRender)]
pub fn time_table_render(props: &TimeTableRenderProps) -> Html {
    html! {
        <pre>
            { props.timetable.to_string_pretty(true, true, false, true, false) }
        </pre>
    }
}
