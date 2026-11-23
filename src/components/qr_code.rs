use qrcode_generator::qr::{Encoder, ErrorCorrection};
use qrcode_generator::Renderer;
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
        // the exact bytes, so nothing a scanner reads differs from what it was given, at the level
        // and with the one-module margin the code always had
        let Ok(symbol) = Encoder::new(ErrorCorrection::Low).boost_error_correction(false).encode_bytes(data.as_bytes()) else {
            return String::new();
        };
        Renderer::new(&symbol, 512).quiet_zone(1).svg_xml_declaration(false).to_svg_string(None::<&str>).unwrap_or_default()
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
            class="d-flex justify-content-center align-items-center w-100 h-100 p-0 overflow-hidden"
        />
    }
}
