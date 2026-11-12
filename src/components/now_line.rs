use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct NowLineProps {
    /// how far down its container the line sits, in percent
    pub top: f64,
}

/// The red line marking the current time, with an arrowhead at either end
#[function_component(NowLine)]
pub fn now_line(props: &NowLineProps) -> Html {
    let top = props.top;

    html! {
        <div style={format!("position: absolute; top: {top}%; left: 0; right: 0; height: 2px; transform: translateY(-1px); background: #ff3b30; z-index: 20; pointer-events: none;")}>
            <div style="position: absolute; left: 0; top: -4px; width: 0; height: 0; border-top: 5px solid transparent; border-bottom: 5px solid transparent; border-left: 7px solid #ff3b30;"></div>
            <div style="position: absolute; right: 0; top: -4px; width: 0; height: 0; border-top: 5px solid transparent; border-bottom: 5px solid transparent; border-right: 7px solid #ff3b30;"></div>
        </div>
    }
}
