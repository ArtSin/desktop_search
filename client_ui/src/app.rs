use std::str::FromStr;

use common_lib::settings::Settings;
use derive_more::Display;
use js_sys::JSON;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use sycamore::prelude::*;
use sycamore::rt::Event;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{HtmlElement, Request, RequestInit, RequestMode, Response};

use crate::{search::Search, settings::Settings, status::Status};

use self::widgets::{StatusDialog, StatusDialogState};

pub mod widgets;

#[derive(Display, PartialEq, Eq, Hash, Clone, Copy)]
enum AppTabs {
    #[display(fmt = "Поиск")]
    Search,
    #[display(fmt = "Статус индексации")]
    Status,
    #[display(fmt = "Настройки")]
    Settings,
}

impl FromStr for AppTabs {
    type Err = std::fmt::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Поиск" => Ok(AppTabs::Search),
            "Статус индексации" => Ok(AppTabs::Status),
            "Настройки" => Ok(AppTabs::Settings),
            _ => Err(std::fmt::Error),
        }
    }
}

#[component]
pub fn App<G: Html>(cx: Scope) -> View<G> {
    // Use default settings until loaded from server
    let settings = create_signal(cx, Settings::default());

    let status_dialog_state = create_signal(cx, StatusDialogState::None);
    let tabs = create_signal(
        cx,
        vec![AppTabs::Search, AppTabs::Status, AppTabs::Settings],
    );
    let curr_tab = create_signal(cx, AppTabs::Search);
    let switch_tab = |event: Event| {
        let event_target = event.target().unwrap();
        let element: &HtmlElement = event_target.dyn_ref::<HtmlElement>().unwrap();
        curr_tab.set(element.text_content().unwrap().parse().unwrap());
    };

    view! { cx,
        nav {
            ul {
                Keyed(
                    iterable=tabs,
                    view=move |cx, x| view! { cx,
                        li {
                            a(on:click=switch_tab,
                                href="javascript:void(0);",
                                class={ if *curr_tab.get().as_ref() == x { "active" } else { "" } }) {
                                (x)
                            }
                        }
                    },
                    key = |x| *x,
                )
            }
        }

        div(style={if *curr_tab.get().as_ref() == AppTabs::Search { "display: block;" } else { "display: none;" }}) {
            Search(status_dialog_state=status_dialog_state)
        }
        div(style={if *curr_tab.get().as_ref() == AppTabs::Status { "display: block;" } else { "display: none;" }}) {
            Status(status_dialog_state=status_dialog_state)
        }
        div(style={if *curr_tab.get().as_ref() == AppTabs::Settings { "display: block;" } else { "display: none;" }}) {
            Settings(settings=settings, status_dialog_state=status_dialog_state)
        }

        StatusDialog(status=status_dialog_state)
    }
}

async fn fetch_response(
    uri: &str,
    method: &str,
    body: Option<&impl Serialize>,
) -> Result<Response, JsValue> {
    let request_body = body
        .map(|x| to_value(x).map_err(Into::<JsValue>::into))
        .transpose()?
        .map(|x| JSON::stringify(&x))
        .transpose()?
        .map(JsValue::from);

    let mut opts = RequestInit::new();
    opts.method(method)
        .mode(RequestMode::SameOrigin)
        .body(request_body.as_ref());

    let request = Request::new_with_str_and_init(uri, &opts)?;
    if request_body.is_some() {
        request.headers().set("Content-Type", "application/json")?;
    }

    let window = web_sys::window().unwrap();
    let response_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let response: Response = response_value.dyn_into().unwrap();
    if response.ok() {
        Ok(response)
    } else {
        Err(JsFuture::from(response.text()?).await?)
    }
}

pub async fn fetch<T>(uri: &str, method: &str, body: Option<&impl Serialize>) -> Result<T, JsValue>
where
    T: for<'de> Deserialize<'de>,
{
    let response = fetch_response(uri, method, body).await?;
    let response_json = JsFuture::from(response.json()?).await?;
    from_value(response_json).map_err(|e| e.into())
}

pub async fn fetch_empty(
    uri: &str,
    method: &str,
    body: Option<&impl Serialize>,
) -> Result<(), JsValue> {
    fetch_response(uri, method, body).await?;
    Ok(())
}
