use common_lib::{actions::PickFolderResult, settings::IndexingDirectory};
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
    pub dir: IndexingDirectory,
}

impl DirectoryItem {
    pub fn new(dir: IndexingDirectory) -> Self {
        Self {
            id: Uuid::new_v4(),
            dir,
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
    let curr_directory = create_signal(cx, IndexingDirectory::default());
    let curr_directory_exclude_str = create_signal(cx, "false".to_owned());
    let curr_directory_empty = create_memo(cx, || curr_directory.get().path.as_os_str().is_empty());

    create_effect(cx, || {
        curr_directory.modify().exclude = curr_directory_exclude_str.get().parse().unwrap();
    });

    let select_item = move |_| {
        spawn_local_scoped(cx, async {
            match pick_folder().await {
                Ok(res) => {
                    if let Some(path) = res.path {
                        curr_directory.modify().path = path;
                    }
                }
                Err(e) => {
                    status_dialog_state.set(StatusDialogState::Error(format!(
                        "❌ Ошибка открытия диалога: {e:#?}",
                    )));
                }
            }
        });
    };

    let add_item = |_| {
        directory_list
            .modify()
            .push(DirectoryItem::new((*curr_directory.get()).clone()));
        curr_directory.set(IndexingDirectory::default());
    };

    view! { cx,
        Keyed(
            iterable=directory_list,
            key=|item| item.id,
            view=move |cx, item| {
                let delete_item = move |_| {
                    directory_list.modify().retain(|x| x.id != item.id);
                };

                view! { cx,
                    div(class="setting") {
                        input(type="text", readonly=true, value=item.dir.path.display()) {}
                        p { (if item.dir.exclude { "Исключено" } else { "Включено" }) }
                        button(type="button", on:click=delete_item) { "➖" }
                    }
                }
            }
        )

        div(class="setting") {
            input(type="text", readonly=true, value=curr_directory.get().path.display()) {}
            button(type="button", on:click=select_item) { "Выбрать..." }
            select(bind:value=curr_directory_exclude_str) {
                option(selected=true, value="false") { "Включить" }
                option(value="true") { "Исключить" }
            }
            button(type="button", on:click=add_item, disabled=*curr_directory_empty.get()) { "➕" }
        }
    }
}
