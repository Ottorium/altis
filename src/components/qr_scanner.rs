use crate::native;
use gloo_timers::future::TimeoutFuture;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    CanvasRenderingContext2d, ContextAttributes2d, HtmlCanvasElement, HtmlVideoElement, MediaStream,
    MediaStreamConstraints, MediaStreamTrack,
};
use yew::prelude::*;

const SCANNING_CLASS: &str = "qr-scanning";
/// pause between searching two webcam frames
const FRAME_INTERVAL_MS: u32 = 200;
/// larger webcam frames are scaled down, they take too long to search
const MAX_FRAME_SIZE: u32 = 960;

#[derive(Properties, PartialEq)]
pub struct QrScannerProps {
    pub on_scan: Callback<String>,
    pub on_error: Callback<String>,
    pub on_cancel: Callback<()>,
}

/// Full screen camera view looking for a QR code, scanning stops once it's removed.
/// The mobile app uses the native scanner, elsewhere the webcam is read through the webview.
#[function_component(QrScanner)]
pub fn qr_scanner(props: &QrScannerProps) -> Html {
    let video_ref = use_node_ref();
    let use_native = native::has_native_scanner();

    {
        let video_ref = video_ref.clone();
        let on_scan = props.on_scan.clone();
        let on_error = props.on_error.clone();
        use_effect_with((), move |_| {
            // cleared once the scanner is removed, the scan then stops and its result is dropped
            let active = Rc::new(Cell::new(true));
            let finished = Rc::new(Cell::new(false));
            {
                let active = active.clone();
                let finished = finished.clone();
                spawn_local(async move {
                    let res = if use_native {
                        scan_native(&active).await
                    } else {
                        scan_webcam(&video_ref, &active).await
                    };
                    finished.set(true);
                    if !active.get() {
                        return;
                    }
                    match res {
                        Ok(text) => on_scan.emit(text),
                        Err(e) => on_error.emit(e),
                    }
                });
            }

            move || {
                active.set(false);
                if use_native && !finished.get() {
                    set_page_transparent(false);
                    spawn_local(async {
                        let _ = native::cancel_scan().await;
                    });
                }
            }
        });
    }

    let on_cancel = {
        let on_cancel = props.on_cancel.clone();
        Callback::from(move |_| on_cancel.emit(()))
    };

    html! {
        <div class={classes!("qr-scanner-overlay", (!use_native).then_some("bg-black"))}>
            if !use_native {
                <video ref={video_ref} class="qr-scanner-video" autoplay=true />
            }
            <div class="position-relative d-flex flex-column justify-content-between align-items-center h-100 p-4">
                <div class="position-relative badge bg-dark fs-6 fw-normal mt-4 text-wrap" style="z-index: 1;">
                    {"Point the camera at the settings QR code"}
                </div>
                <div class="qr-scanner-frame"></div>
                <button type="button" class="position-relative btn btn-light btn-lg mb-4" style="z-index: 1;" onclick={on_cancel}>
                    {"Cancel"}
                </button>
            </div>
        </div>
    }
}

/// The native scanner shows the camera behind the webview, so the page is made see-through while scanning
fn set_page_transparent(transparent: bool) {
    let Some(root) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.document_element()) else { return };
    let classes = root.class_list();
    let _ = if transparent { classes.add_1(SCANNING_CLASS) } else { classes.remove_1(SCANNING_CLASS) };
}

async fn scan_native(active: &Cell<bool>) -> Result<String, String> {
    native::request_camera_permission().await?;
    if !active.get() {
        return Err("Cancelled".to_string());
    }
    set_page_transparent(true);
    let res = native::scan_qr_code().await;
    set_page_transparent(false);
    res
}

async fn scan_webcam(video_ref: &NodeRef, active: &Cell<bool>) -> Result<String, String> {
    let stream = open_webcam().await?;
    let res = find_qr_code(video_ref, &stream, active).await;
    for track in stream.get_tracks() {
        track.unchecked_into::<MediaStreamTrack>().stop();
    }
    res
}

async fn open_webcam() -> Result<MediaStream, String> {
    let devices = web_sys::window().ok_or("No window found")?
        .navigator().media_devices().map_err(native::error_message)?;
    let constraints = MediaStreamConstraints::new();
    let video = js_sys::JSON::parse(r#"{"facingMode":"environment","width":{"ideal":1280},"height":{"ideal":720}}"#)
        .map_err(native::error_message)?;
    constraints.set_video(&video);
    let promise = devices.get_user_media_with_constraints(&constraints).map_err(native::error_message)?;
    let stream = JsFuture::from(promise).await.map_err(|e| format!("No camera access: {}", native::error_message(e)))?;
    Ok(stream.unchecked_into())
}

async fn find_qr_code(video_ref: &NodeRef, stream: &MediaStream, active: &Cell<bool>) -> Result<String, String> {
    let video = video_ref.cast::<HtmlVideoElement>().ok_or("The camera view is missing")?;
    video.set_src_object(Some(stream));
    if let Ok(playing) = video.play() {
        let _ = JsFuture::from(playing).await;
    }

    let canvas: HtmlCanvasElement = web_sys::window().and_then(|w| w.document()).ok_or("No document found")?
        .create_element("canvas").map_err(native::error_message)?
        .unchecked_into();
    let options = ContextAttributes2d::new();
    options.set_will_read_frequently(true);
    let ctx: CanvasRenderingContext2d = canvas.get_context_with_context_options("2d", &options)
        .map_err(native::error_message)?
        .ok_or("Canvas isn't supported")?
        .unchecked_into();

    while active.get() {
        if let Some(text) = read_frame(&video, &canvas, &ctx) {
            return Ok(text);
        }
        TimeoutFuture::new(FRAME_INTERVAL_MS).await;
    }
    Err("Cancelled".to_string())
}

fn read_frame(video: &HtmlVideoElement, canvas: &HtmlCanvasElement, ctx: &CanvasRenderingContext2d) -> Option<String> {
    let (video_w, video_h) = (video.video_width(), video.video_height());
    if video_w == 0 || video_h == 0 {
        // no frame yet
        return None;
    }
    let scale = (MAX_FRAME_SIZE as f64 / video_w.max(video_h) as f64).min(1.0);
    let (w, h) = ((video_w as f64 * scale) as u32, (video_h as f64 * scale) as u32);
    if canvas.width() != w || canvas.height() != h {
        canvas.set_width(w);
        canvas.set_height(h);
    }

    ctx.draw_image_with_html_video_element_and_dw_and_dh(video, 0.0, 0.0, w as f64, h as f64).ok()?;
    let pixels = ctx.get_image_data(0.0, 0.0, w as f64, h as f64).ok()?.data();
    decode_qr(w as usize, h as usize, &pixels)
}

/// Looks for a QR code in an RGBA image
fn decode_qr(w: usize, h: usize, rgba: &[u8]) -> Option<String> {
    let mut img = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| {
        let i = (y * w + x) * 4;
        ((rgba[i] as u16 * 77 + rgba[i + 1] as u16 * 150 + rgba[i + 2] as u16 * 29) >> 8) as u8
    });
    img.detect_grids().into_iter().find_map(|grid| grid.decode().ok()).map(|(_, text)| text)
}

#[cfg(test)]
mod tests {
    use super::decode_qr;
    use qrcode_generator::QrCodeEcc;

    #[test]
    fn decodes_generated_qr_code() {
        let text = r##"{"visual_settings":{"force_ascii_timetable":false,"subject_color_overrides":{"M":"#4a90e2"}}}"##;
        let matrix = qrcode_generator::to_matrix(text, QrCodeEcc::Low).unwrap();
        let (scale, border) = (4, 16);
        let size = matrix.len() * scale + border * 2;
        let mut rgba = vec![255u8; size * size * 4];
        for (y, row) in matrix.iter().enumerate() {
            for (x, _) in row.iter().enumerate().filter(|(_, dark)| **dark) {
                for py in 0..scale {
                    for px in 0..scale {
                        let i = ((border + y * scale + py) * size + border + x * scale + px) * 4;
                        rgba[i..i + 3].fill(0);
                    }
                }
            }
        }
        assert_eq!(decode_qr(size, size, &rgba).as_deref(), Some(text));
    }
}
