use std::{borrow::Cow, str::FromStr, sync::OnceLock};

use common_lib::{settings::Settings, ClientTranslation};
use derive_more::Display;
use fluent_bundle::{bundle::FluentBundle, FluentArgs, FluentResource};
use intl_memoizer::concurrent::IntlLangMemoizer;
use js_sys::JSON;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use sycamore::prelude::*;
use sycamore::rt::Event;
use unic_langid::LanguageIdentifier;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{HtmlElement, Request, RequestInit, RequestMode, Response};

use crate::{search::Search, settings::Settings, status::Status};

use self::widgets::{StatusDialog, StatusDialogState};

pub mod widgets;

static TRANSLATION: OnceLock<FluentBundle<FluentResource, IntlLangMemoizer>> = OnceLock::new();

#[derive(Display, PartialEq, Eq, Hash, Clone, Copy)]
enum AppTabs {
    #[display(fmt = "search_tab")]
    Search,
    #[display(fmt = "indexing_status_tab")]
    IndexingStatus,
    #[display(fmt = "settings_tab")]
    Settings,
}

impl FromStr for AppTabs {
    type Err = std::fmt::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "search_tab" => Ok(AppTabs::Search),
            "indexing_status_tab" => Ok(AppTabs::IndexingStatus),
            "settings_tab" => Ok(AppTabs::Settings),
            _ => Err(std::fmt::Error),
        }
    }
}

#[component]
pub async fn App<G: Html>(cx: Scope<'_>) -> View<G> {
    assert!(TRANSLATION.set(load_translation().await).is_ok());

    let document = web_sys::window()
        .expect("`window` not found")
        .document()
        .expect("`document` not found");
    document
        .document_element()
        .expect("`html` not found")
        .set_attribute("lang", &get_translation("lang_code", None))
        .unwrap();
    document.set_title(&get_translation("title", None));

    // Use default settings until loaded from server
    let settings = create_signal(cx, Settings::default());

    let status_dialog_state = create_signal(cx, StatusDialogState::None);
    let tabs = create_signal(
        cx,
        vec![AppTabs::Search, AppTabs::IndexingStatus, AppTabs::Settings],
    );
    let curr_tab = create_signal(cx, AppTabs::Search);
    let switch_tab = |event: Event| {
        let event_target = event.target().unwrap();
        let element: &HtmlElement = event_target.dyn_ref::<HtmlElement>().unwrap();
        curr_tab.set(element.id().parse().unwrap());
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
                                id=x,
                                class={ if *curr_tab.get().as_ref() == x { "active" } else { "" } }) {
                                (get_translation(x.to_string(), None))
                            }
                        }
                    },
                    key = |x| *x,
                )
            }
        }

        div(style={if *curr_tab.get().as_ref() == AppTabs::Search { "display: block;" } else { "display: none;" }}) {
            Search(settings=settings, status_dialog_state=status_dialog_state)
        }
        div(style={if *curr_tab.get().as_ref() == AppTabs::IndexingStatus { "display: block;" } else { "display: none;" }}) {
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

async fn load_translation() -> FluentBundle<FluentResource, IntlLangMemoizer> {
    let translation_data: ClientTranslation = fetch("/client_translation", "GET", None::<&()>)
        .await
        .unwrap();

    let lang_id: LanguageIdentifier = translation_data.lang_id.parse().unwrap();
    let mut bundle = FluentBundle::new_concurrent(vec![lang_id]);
    let resource = FluentResource::try_new(translation_data.content).unwrap();
    bundle.add_resource(resource).unwrap();
    bundle
}

pub fn get_translation<'a, S: AsRef<str>>(
    message_id: S,
    args: Option<&'a FluentArgs<'_>>,
) -> Cow<'a, str> {
    let bundle = TRANSLATION.get().unwrap();
    let message = bundle
        .get_message(message_id.as_ref())
        .expect(message_id.as_ref());
    let mut errors = Vec::new();
    bundle.format_pattern(
        message.value().expect(message_id.as_ref()),
        args,
        &mut errors,
    )
}
