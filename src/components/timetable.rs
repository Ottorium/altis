use crate::untis_client;
use crate::untis_week::Week;
use yew::prelude::*;
use yew::suspense::use_future;

#[function_component(TimetableComponent)]
pub fn timetable() -> HtmlResult {
    let res = use_future(|| async {
        untis_client::get_classes(Week::current()).await
    })?;

    let result_html = match &*res {
        Ok(data) => html! {
            <pre>{ format!("{:#?}", data.iter().map(|c| &c.name).collect::<Vec<_>>()) }</pre>
        },
        Err(failure) => html! { { failure.to_string() } },
    };

    Ok(html! {
        <div>
            { result_html }
        </div>
    })
}