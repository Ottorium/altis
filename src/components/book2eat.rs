use crate::book2eat::client::{get_b2e_token, get_menu};
use crate::components::qr_code::QrCode;
use crate::data_models::response_models::book2eat::Meal;
use crate::untis::untis_week::Week;
use chrono::Local;
use yew::prelude::*;
use yew::suspense::use_future_with;

#[function_component(Book2EatComponent)]
pub fn book2eat() -> HtmlResult {
    let reload_trigger = use_state(|| 0);

    let res = use_future_with(*reload_trigger, |_| async move {
        let (user, token) = get_b2e_token()
            .await
            .map_err(|e| e.to_string())?;

        get_menu(Week::current(), user, token)
            .await
            .map_err(|e| e.to_string())
    })?;

    match &*res {
        Ok(menu_response) => {
            let today = Local::now().format("%Y-%m-%d").to_string();

            let todays_meals: Vec<_> = menu_response.data.menue.values()
                .filter(|meal| meal.date == today).cloned()
                .collect();


            Ok(html! {
                <div class="container pt-4 pb-2 d-flex flex-column align-items-center w-100 h-100">
                    <div class="d-flex justify-content-center align-items-center bg-light rounded p-4 mb-4" style="aspect-ratio: 1 / 1;">
                        <QrCode data={menu_response.data.qr_code.clone()} />
                    </div>

                    <div class="d-flex flex-column flex-grow-1 justify-content-evenly bg-black w-100 p-2 mx-5 g-2 rounded-4 overflow-auto">
                        { for sort_meals(todays_meals.clone()).iter().enumerate().map(|(index, meal)| {
                            let is_last = index == todays_meals.len() - 1;
                            let border_class = if is_last { "" } else { "border-bottom border-secondary" };

                            html! {
                                <div class={classes!(border_class, "shadow-sm", "text-center", "text-white", "py-2", "w-100")}>
                                    <div class="fw-bold">
                                        { &meal.name }
                                    </div>
                                    <div class="text-secondary">
                                        { &meal.typ_name }
                                    </div>
                                </div>
                            }
                        }) }
                    </div>
                </div>
            })
        }
        Err(err_msg) => Ok(html! {
            <div class="container mt-5">
                <div class="alert alert-danger shadow-sm">
                    <strong>{"Error:"}</strong> { format!(" {}", err_msg) }
                </div>
            </div>
        })
    }
}

fn sort_meals(todays_meals: Vec<Meal>) -> Vec<Meal> {
    let mut todays_meals = todays_meals.clone();
    todays_meals.sort_by_key(|x| { x.typ_name.clone().chars().next() });
    todays_meals.reverse();
    todays_meals
}