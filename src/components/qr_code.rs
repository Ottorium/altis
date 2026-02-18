use qrcode_generator::QrCodeEcc;
use web_sys::Element;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct QrProps {
    pub data: String,
}

#[function_component(QrCode)]
pub fn qr_code(props: &QrProps) -> Html {
    let node_ref = use_node_ref();

    let qr_svg_string = use_memo(props.data.clone(), |data| {
        qrcode_generator::to_svg_to_string(data, QrCodeEcc::Low, 512, None::<&str>)
            .unwrap_or_default()
    });

    use_effect_with(qr_svg_string.clone(), {
        let node_ref = node_ref.clone();
        move |svg_content| {
            if let Some(div) = node_ref.cast::<Element>() {
                div.set_inner_html(svg_content);

                if let Some(svg) = div.query_selector("svg").ok().flatten() {
                    if let Some(rect) = svg.query_selector("rect").ok().flatten() {
                        rect.remove();
                    }
                    let _ = svg.set_attribute("viewBox", "0 0 512 512");
                    let _ = svg.remove_attribute("width");
                    let _ = svg.remove_attribute("height");
                    let _ = svg.set_attribute("style", "width: 100%; height: 100%;");
                }
            }
            || ()
        }
    });

    html! {
        <div
            ref={node_ref}
            class="d-flex justify-content-center align-items-center w-100 h-100 p-0"
            style="overflow: hidden;"
        />
    }
}
