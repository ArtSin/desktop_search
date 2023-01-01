use std::path::PathBuf;

use common_lib::actions::PickFolderResult;
use sycamore::{futures::spawn_local_scoped, prelude::*};
use uuid::Uuid;
use wasm_bindgen::JsValue;

use crate::app::{fetch, widgets::StatusDialogState};

#[derive(Prop)]
pub struct TextSettingProps<'a> {
    pub id: &'static str,
    pub label: &'static str,
    pub value: &'a Signal<String>,
    pub valid: &'a ReadSignal<bool>,
}

#[component]
pub fn TextSetting<'a, G: Html>(cx: Scope<'a>, props: TextSettingProps<'a>) -> View<G> {
    let value = props.value;
    view! { cx,
        div(class="setting") {
            label(for=props.id) { (props.label) }
            input(type="text", id=props.id, name=props.id, bind:value=value) {}
            (if *props.valid.get() { "✅" } else { "❌" })
        }
    }
}

#[derive(Prop)]
pub struct NumberSettingProps<'a> {
    pub id: &'static str,
    pub label: &'static str,
    pub value: &'a Signal<String>,
    pub valid: &'a ReadSignal<bool>,
}

#[component]
pub fn NumberSetting<'a, G: Html>(cx: Scope<'a>, props: NumberSettingProps<'a>) -> View<G> {
    let value = props.value;
    view! { cx,
        div(class="setting") {
            label(for=props.id) { (props.label) }
            input(type="text", size=10, id=props.id, name=props.id, bind:value=value) {}
            (if *props.valid.get() { "✅" } else { "❌" })
        }
    }
}

#[derive(Prop)]
pub struct CheckboxSettingProps<'a> {
    pub id: &'static str,
    pub label: &'static str,
    pub value: &'a Signal<bool>,
}

#[component]
pub fn CheckboxSetting<'a, G: Html>(cx: Scope<'a>, props: CheckboxSettingProps<'a>) -> View<G> {
    let value = props.value;
    view! { cx,
        div(class="checkbox_setting") {
            label(for=props.id) { (props.label) }
            input(type="checkbox", id=props.id, name=props.id, bind:checked=value) {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryItem {
    pub id: Uuid,
    pub path: PathBuf,
}

impl DirectoryItem {
    pub fn new(path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            path,
        }
    }
}

async fn pick_folder() -> Result<PickFolderResult, JsValue> {
    fetch("/pick_folder", "POST", None::<&()>).await
}

#[component(inline_props)]
pub fn DirectoryList<'a, G: Html>(
    cx: Scope<'a>,
    directory_list: &'a Signal<Vec<DirectoryItem>>,
    status_dialog_state: &'a Signal<StatusDialogState>,
) -> View<G> {
    let curr_directory = create_signal(cx, PathBuf::new());
    let curr_directory_empty = create_memo(cx, || curr_directory.get().as_os_str().is_empty());

    let select_item = move |_| {
        spawn_local_scoped(cx, async {
            match pick_folder().await {
                Ok(res) => {
                    if let Some(path) = res.path {
                        curr_directory.set(path);
                    }
                }
                Err(e) => {
                    status_dialog_state.set(StatusDialogState::Error(format!(
                        "❌ Ошибка открытия диалога: {:#?}",
                        e
                    )));
                }
            }
        });
    };

    let add_item = |_| {
        directory_list
            .modify()
            .push(DirectoryItem::new((*curr_directory.get()).clone()));
        curr_directory.set(PathBuf::new());
    };

    view! { cx,
        Keyed(
            iterable=directory_list,
            view=move |cx, item| {
                let delete_item = move |_| {
                    directory_list.modify().retain(|x| x.id != item.id);
                };

                view! { cx,
                    div(class="setting") {
                        input(type="text", readonly=true, value=item.path.display()) {}
                        button(type="button", on:click=delete_item) { "➖" }
                    }
                }
            },
            key=|item| item.id,
        )

        div(class="setting") {
            input(type="text", readonly=true, value=curr_directory.get().display()) {}
            button(type="button", on:click=select_item) { "Выбрать..." }
            button(type="button", on:click=add_item, disabled=*curr_directory_empty.get()) { "➕" }
        }
    }
}
