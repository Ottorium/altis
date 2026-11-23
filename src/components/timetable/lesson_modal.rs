use crate::components::modal::Modal;
use altis_core::models::timetable::{ChangeStatus, Entity, LessonBlock, LessonStatus, Tracked};
use web_sys::MouseEvent;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LessonModalProps {
    /// the lessons of the block that was clicked
    pub lessons: Vec<LessonBlock>,
    pub on_close: Callback<()>,
    /// a teacher, class or room was clicked, to show its timetable
    pub on_entity_click: Callback<Entity>,
}

/// The details of the lessons that take place at the same time
#[function_component(LessonModal)]
pub fn lesson_modal(props: &LessonModalProps) -> Html {
    html! {
        <Modal title="Time Block Details" on_close={props.on_close.clone()} large=true body_class="p-4 custom-scrollbar" limit_body_height=true>
            { for props.lessons.iter().filter(|lesson| !lesson.is_break()).map(|lesson| render_lesson(lesson, &props.on_entity_click)) }
        </Modal>
    }
}

/// What became of the lesson, and the colour that is told in. `None` for a lesson that takes place
/// as planned.
fn status_badge(lesson: &LessonBlock) -> Option<(String, &'static str)> {
    let (label, class) = match &lesson.status {
        LessonStatus::Cancelled => ("Cancelled", "bg-danger"),
        LessonStatus::Changed => ("Changed", "bg-info text-dark"),
        LessonStatus::Additional => ("Additional", "bg-success"),
        LessonStatus::Other(status) if status.is_empty() || status == "REGULAR" => return None,
        LessonStatus::Other(status) => return Some((status.clone(), "bg-primary text-black")),
    };
    Some((label.to_string(), class))
}

fn render_lesson(lesson: &LessonBlock, on_entity_click: &Callback<Entity>) -> Html {
    let hex = lesson.color_hex.trim_start_matches('#');
    let border_style = format!("border-left: 5px solid #{hex} !important; background-color: #2b3035;");
    // an entity without a name is one Untis may not show, and then none of them are listed
    let all_named = !lesson.entities.iter().any(|entity| entity.inner.name().is_empty());
    let cancelled = lesson.is_cancelled();

    html! {
        <div class="card mb-3 shadow-sm border-0" style={border_style}>
            <div class="card-body text-light">
                <h5 class="card-title fw-bold mb-1 text-white">{ lesson.kind.as_str() }</h5>

                <div class="d-flex flex-wrap align-items-center gap-2 mb-3">
                    if lesson.is_exam() {
                        <span class="badge bg-warning text-dark">{"Exam"}</span>
                    }
                    if let Some((label, class)) = status_badge(lesson) {
                        <span class={format!("badge {class}")}>{ label }</span>
                    }
                    <span class="text-secondary small">
                        <i class="bi bi-clock me-1"></i>
                        { format!("{} - {}", lesson.time_range.start.format("%H:%M"), lesson.time_range.end.format("%H:%M")) }
                    </span>
                </div>

                if all_named {
                    <div class="d-flex flex-wrap gap-2">
                        { for lesson.entities.iter().map(|entity| render_entity(entity, cancelled, on_entity_click)) }
                    </div>
                }

                if !lesson.icons.is_empty() {
                    <div class="mb-2 text-info">
                        { for lesson.icons.iter().map(|icon| html! {
                            <i class={format!("{} me-3", icon)}></i>
                        })}
                    </div>
                }

                { for lesson.texts.iter().filter(|texts| texts.values().any(|text| !text.is_empty())).map(|texts| html! {
                    <div class="mt-2 pt-2 border-top border-secondary">
                        { for texts.iter().filter(|(_, text)| !text.is_empty()).map(|(name, text)| html! {
                            <div class="small text-secondary">
                                <strong class="text-light">{ format!("{name}: ") }</strong>
                                { text }
                            </div>
                        })}
                    </div>
                })}

                if !lesson.link.is_empty() {
                    <div class="mt-3">
                        <a href={lesson.link.clone()} target="_blank" rel="noopener noreferrer" class="btn btn-sm btn-outline-info p-1 px-2 text-decoration-none">
                            <i class="bi bi-link-45deg me-1"></i>{"View Attachment"}
                        </a>
                    </div>
                }
            </div>
        </div>
    }
}

/// What a change did to a teacher, class, room or subject of the lesson, in words. `None` for
/// one that is there as planned.
fn change_label(entity: &Tracked<Entity>, cancelled: bool) -> Option<&'static str> {
    let is_teacher = entity.inner.is_teacher();
    match entity.status {
        ChangeStatus::Removed if is_teacher => Some("absent"),
        ChangeStatus::Removed => Some("removed"),
        // a lesson that doesn't take place has no one there, whoever was planned
        _ if cancelled => Some("cancelled"),
        ChangeStatus::Changed if is_teacher => Some("substitute"),
        ChangeStatus::Changed => Some("instead"),
        ChangeStatus::New => Some("added"),
        ChangeStatus::Regular => None,
    }
}

/// A chip for a teacher, class, room, subject or note of the lesson. The ones that have a
/// timetable of their own open it when clicked.
fn render_entity(entity: &Tracked<Entity>, cancelled: bool, on_entity_click: &Callback<Entity>) -> Html {
    let (background_class, icon) = match entity.inner {
        Entity::Teacher(_) => ("bg-primary", "bi-person-badge"),
        Entity::Class(_) => ("bg-success", "bi-people"),
        Entity::Room(_) => ("bg-warning text-dark", "bi-geo-alt"),
        Entity::Subject(_) => ("bg-info text-dark", "bi-book"),
        Entity::Info(_) => ("bg-secondary", "bi-info-circle"),
    };

    let has_timetable = entity.inner.kind().is_some();
    let onclick = has_timetable.then(|| {
        let (on_entity_click, entity) = (on_entity_click.clone(), entity.inner.clone());
        Callback::from(move |_: MouseEvent| on_entity_click.emit(entity.clone()))
    });

    // what is not there is struck through and faded, so what is stands out
    let gone = entity.status == ChangeStatus::Removed || cancelled;
    let label = change_label(entity, cancelled);
    let label_class = match entity.status {
        ChangeStatus::Removed => "bg-danger text-white",
        _ if cancelled => "bg-danger text-white",
        ChangeStatus::Changed | ChangeStatus::New => "bg-dark text-success",
        ChangeStatus::Regular => "",
    };

    html! {
        <div class={format!("d-inline-flex align-items-center px-2 py-1 rounded-1 text-black shadow-sm {background_class}")}
             style={format!("width: fit-content; font-size: 0.85rem; min-width: max-content;{}{}", if has_timetable { " cursor: pointer;" } else { "" }, if gone { " opacity: 0.6;" } else { "" })}
             title={has_timetable.then_some("Show timetable")}
             {onclick}>
            <i class={format!("bi {icon} me-2")}></i>
            if gone {
                <del><strong style="letter-spacing: 0.3px;">{ entity.inner.name() }</strong></del>
            } else {
                <strong style="letter-spacing: 0.3px;">{ entity.inner.name() }</strong>
            }
            if let Some(label) = label {
                <span class={format!("badge ms-2 {label_class}")} style="font-size: 0.7rem;">{ label }</span>
            }
            if has_timetable {
                <i class="bi bi-chevron-right ms-1"></i>
            }
        </div>
    }
}
