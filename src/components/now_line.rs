use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct NowLineProps {
    /// how far down its container the line sits, in percent
    pub top: f64,
}

/// The red line marking the current time, with an arrowhead at either end (see `.now-line`)
#[function_component(NowLine)]
pub fn now_line(props: &NowLineProps) -> Html {
    html! { <div class="now-line" style={format!("top: {}%;", props.top)}></div> }
}
