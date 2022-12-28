use std::str::FromStr;

use common_lib::settings::{ClientSettings, ServerSettings};
use derive_more::Display;
use sycamore::prelude::*;
use sycamore::rt::Event;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::HtmlElement;

use crate::{search::Search, settings::Settings, status::Status};

use self::widgets::{StatusDialog, StatusDialogState};

pub mod widgets;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "tauri"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

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
    let client_settings = create_signal(cx, ClientSettings::default());
    let server_settings = create_signal(cx, ServerSettings::default());

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
            Search(client_settings=client_settings, status_dialog_state=status_dialog_state)
        }
        div(style={if *curr_tab.get().as_ref() == AppTabs::Status { "display: block;" } else { "display: none;" }}) {
            Status(status_dialog_state=status_dialog_state)
        }
        div(style={if *curr_tab.get().as_ref() == AppTabs::Settings { "display: block;" } else { "display: none;" }}) {
            Settings(client_settings=client_settings, server_settings=server_settings,
                status_dialog_state=status_dialog_state)
        }

        StatusDialog(status=status_dialog_state)
    }
}
