use crate::data_models::clean_models::untis::{Entity, LessonBlock};
use web_sys::MouseEvent;
use yew::{function_component, html, Callback, Html, Properties};

#[derive(Properties, PartialEq)]
pub struct GroupModalProps {
    pub lessons: Vec<LessonBlock>,
    pub on_close: Callback<()>,
}

#[function_component(GroupDetailModal)]
pub fn group_detail_modal(props: &GroupModalProps) -> Html {
    let on_close = props.on_close.clone();

    html! {
        <div class="modal d-block" style="background: rgba(0,0,0,0.85); z-index: 1050;" onclick={
            let on_close = on_close.clone();
            move |_| on_close.emit(())
        }>
            <div class="modal-dialog modal-lg modal-dialog-centered" onclick={|e: MouseEvent| e.stop_propagation()}>
                <div class="modal-content border-primary shadow-lg bg-dark text-light">
                    <div class="modal-header border-primary bg-black text-white">
                        <h5 class="modal-title fw-bold">{"Time Block Details"}</h5>
                        <button type="button" class="btn-close btn-close-white" onclick={
                            let on_close = on_close.clone();
                            move |_| on_close.emit(())
                        }></button>
                    </div>
                    <div class="modal-body p-4 custom-scrollbar" style="max-height: 80vh; overflow-y: auto; background-color: #1a1d20;">
                        { for props.lessons.iter().filter(|l| l.r#type != "Break").map(|l| {
                            let border_style = format!("border-left: 5px solid #{} !important; background-color: #2b3035;", l.color_hex);
                            html! {
                                <div class="card mb-3 shadow-sm border-0" style={border_style}>
                                    <div class="card-body text-light">
                                        <h5 class="card-title fw-bold mb-1 text-white">{ &l.r#type }</h5>

                                        <div class="d-flex align-items-center gap-2 mb-3">
                                            if !l.status.is_empty() {
                                                <span class="badge bg-primary text-black">
                                                    { &l.status }
                                                </span>
                                            }
                                            <span class="text-secondary small">
                                                <i class="bi bi-clock me-1"></i>
                                                { format!("{} - {}", l.time_range.start.format("%H:%M"), l.time_range.end.format("%H:%M")) }
                                            </span>
                                        </div>

                                        if !l.entities.iter().any(|e| e.inner.name().is_empty()) {
                                            <div class="d-flex flex-wrap gap-2">
                                                { for l.entities.iter().map(|entity| {
                                                    let (bg_class, icon) = match entity.inner {
                                                        Entity::Teacher(_) => ("bg-primary", "bi-person-badge"),
                                                        Entity::Class(_) => ("bg-success", "bi-people"),
                                                        Entity::Room(_) => ("bg-warning text-dark", "bi-geo-alt"),
                                                        Entity::Subject(_) => ("bg-info text-dark", "bi-book"),
                                                        Entity::Info(_) => ("bg-secondary", "bi-info-circle"),
                                                    };

                                                    html! {
                                                        <div class={format!("d-inline-flex align-items-center px-2 py-1 rounded-1 text-black shadow-sm {}", bg_class)}
                                                             style="width: fit-content; font-size: 0.85rem; min-width: max-content;">
                                                            <i class={format!("bi {} me-2", icon)}></i>
                                                            <strong style="letter-spacing: 0.3px;">{ entity.inner.name() }</strong>
                                                        </div>
                                                    }
                                                })}
                                            </div>
                                        }

                                        if !l.icons.is_empty() {
                                            <div class="mb-2 text-info">
                                                { for l.icons.iter().map(|icon| html! {
                                                    <i class={format!("{} me-3", icon)}></i>
                                                })}
                                            </div>
                                        }

                                        { for l.texts.iter().map(|map| {
                                            html! {
                                                <div class="mt-2 pt-2 border-top border-secondary">
                                                    { for map.iter().map(|(k, v)| {
                                                        if !v.is_empty() {
                                                            html! {
                                                                <div class="small text-secondary">
                                                                    <strong class="text-light">{format!("{}: ", k)}</strong>
                                                                    {v}
                                                                </div>
                                                            }
                                                        } else {
                                                            html! {}
                                                        }
                                                    })}
                                                </div>
                                            }
                                        })}

                                        if !l.link.is_empty() {
                                            <div class="mt-3">
                                                <a href={l.link.clone()} target="_blank" rel="noopener noreferrer" class="btn btn-sm btn-outline-info p-1 px-2 text-decoration-none">
                                                    <i class="bi bi-link-45deg me-1"></i>{"View Attachment"}
                                                </a>
                                            </div>
                                        }
                                    </div>
                                </div>
                            }
                        })}
                    </div>
                </div>
            </div>
        </div>
    }
}
