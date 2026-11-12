use altis_core::models::absences::Absence;
use web_sys::MouseEvent;
use yew::prelude::*;

const DATE_FORMAT: &str = "%a, %d.%m.%Y";

#[derive(Properties, PartialEq)]
pub struct AbsenceCardProps {
    pub absence: Absence,
    /// whether the school lets students withdraw absences at all
    pub deletable: bool,
    pub on_edit: Callback<Absence>,
    pub on_delete: Callback<Absence>,
}

/// How an absence stands with the school, as the classes and the text of its badge. The school's
/// own wording goes first; without one it is just excused or still open.
fn excuse_badge(absence: &Absence) -> (&'static str, String) {
    match (absence.is_excused, absence.excuse_status.as_str()) {
        (true, "") => ("bg-success text-dark", "Excused".to_string()),
        (true, status) => ("bg-success text-dark", status.to_string()),
        (false, "") => ("bg-warning text-dark", "Open".to_string()),
        (false, status) => ("bg-danger", status.to_string()),
    }
}

/// One absence in the list: when it was, why, who entered it and whether it is excused
#[function_component(AbsenceCard)]
pub fn absence_card(props: &AbsenceCardProps) -> Html {
    let absence = &props.absence;
    let (badge_class, badge_text) = excuse_badge(absence);

    let date = match absence.start_date == absence.end_date {
        true => absence.start_date.format(DATE_FORMAT).to_string(),
        false => format!("{} - {}", absence.start_date.format(DATE_FORMAT), absence.end_date.format(DATE_FORMAT)),
    };

    let emit_absence = |callback: &Callback<Absence>| {
        let absence = absence.clone();
        callback.reform(move |_: MouseEvent| absence.clone())
    };

    html! {
        <div class="card border-0 shadow-sm" style="background-color: #1a1a1a;">
            // `.card` resets the text colour to the light theme's, so the card says its own
            <div class="card-body text-light d-flex align-items-start gap-3 py-3">
                <div class="flex-grow-1" style="min-width: 0;">
                    <div class="d-flex flex-wrap align-items-center gap-2 mb-1">
                        <span class="fw-semibold">{ date }</span>
                        <span class={classes!("badge", badge_class)}>{ badge_text }</span>
                    </div>
                    <div class="text-secondary small">
                        <i class="bi bi-clock me-1"></i>
                        { format!("{} - {}", absence.start_time.format("%H:%M"), absence.end_time.format("%H:%M")) }
                        if !absence.reason.is_empty() {
                            <span class="ms-3"><i class="bi bi-tag me-1"></i>{ &absence.reason }</span>
                        }
                        if !absence.created_user.is_empty() {
                            <span class="ms-3"><i class="bi bi-person me-1"></i>{ &absence.created_user }</span>
                        }
                    </div>
                    if !absence.text.is_empty() {
                        <div class="mt-2 small message-content">{ &absence.text }</div>
                    }
                </div>
                // the same absences a student may withdraw are the ones they may still change
                if absence.can_edit {
                    <div class="d-flex gap-2 flex-shrink-0">
                        <button class="btn btn-sm btn-outline-primary" title="Edit" onclick={emit_absence(&props.on_edit)}>
                            <i class="bi bi-pencil"></i>
                        </button>
                        if props.deletable {
                            <button class="btn btn-sm btn-outline-danger" title="Withdraw" onclick={emit_absence(&props.on_delete)}>
                                <i class="bi bi-trash"></i>
                            </button>
                        }
                    </div>
                }
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

    fn absence(is_excused: bool, excuse_status: &str) -> Absence {
        Absence {
            id: 1,
            start_date: NaiveDate::default(),
            end_date: NaiveDate::default(),
            start_time: NaiveTime::default(),
            end_time: NaiveTime::default(),
            reason: String::new(),
            reason_id: 0,
            text: String::new(),
            created_user: String::new(),
            can_edit: false,
            excuse_status: excuse_status.to_string(),
            is_excused,
        }
    }

    #[test]
    fn the_badge_uses_the_schools_wording_where_there_is_one() {
        assert_eq!(excuse_badge(&absence(true, "")), ("bg-success text-dark", "Excused".to_string()));
        assert_eq!(excuse_badge(&absence(true, "entsch.")), ("bg-success text-dark", "entsch.".to_string()));
        assert_eq!(excuse_badge(&absence(false, "")), ("bg-warning text-dark", "Open".to_string()));
        assert_eq!(excuse_badge(&absence(false, "nicht entsch.")), ("bg-danger", "nicht entsch.".to_string()));
    }
}
