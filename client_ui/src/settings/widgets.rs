use std::{fmt::Display, ops::DerefMut, str::FromStr};

use common_lib::{actions::PickFolderResult, settings::IndexingDirectory};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use uuid::Uuid;
use wasm_bindgen::JsValue;

use crate::app::{fetch, widgets::StatusDialogState};

#[derive(Prop)]
pub struct SimpleTextSettingProps<'a> {
    pub id: &'static str,
    pub label: &'static str,
    pub value: &'a Signal<String>,
}

#[component]
pub fn SimpleTextSetting<'a, G: Html>(cx: Scope<'a>, props: SimpleTextSettingProps<'a>) -> View<G> {
    let value = props.value;
    view! { cx,
        div(class="setting") {
            label(for=props.id) { (props.label) }
            input(type="text", id=props.id, name=props.id, bind:value=value) {}
        }
    }
}

#[derive(Prop)]
pub struct TextSettingProps<'a, T, F> {
    pub id: &'static str,
    pub label: &'static str,
    pub parse: F,
    pub value: &'a Signal<T>,
    pub valid: &'a Signal<bool>,
}

#[component]
pub fn TextSetting<'a, T, E, F, G>(cx: Scope<'a>, props: TextSettingProps<'a, T, F>) -> View<G>
where
    T: Display,
    F: Fn(&str) -> Result<T, E> + 'a,
    G: Html,
{
    let value_str = create_signal(cx, props.value.to_string());

    create_effect(cx, move || match (props.parse)(&value_str.get()) {
        Ok(x) => {
            props.valid.set(true);
            props.value.set_silent(x);
        }
        Err(_) => {
            props.valid.set(false);
        }
    });
    create_effect(cx, || {
        value_str.set(props.value.get().to_string());
    });

    view! { cx,
        div(class="setting") {
            label(for=props.id) { (props.label) }
            input(type="text", id=props.id, name=props.id, bind:value=value_str) {}
            (if *props.valid.get() { "✅" } else { "❌" })
        }
    }
}

#[derive(Prop)]
pub struct NumberSettingProps<'a, T> {
    pub id: &'static str,
    pub label: &'static str,
    pub min: T,
    pub max: T,
    pub value: &'a Signal<T>,
    pub valid: &'a Signal<bool>,
}

#[component]
pub fn NumberSetting<'a, T, G>(cx: Scope<'a>, props: NumberSettingProps<'a, T>) -> View<G>
where
    T: Copy + FromStr + Display + PartialOrd,
    <T as FromStr>::Err: Display,
    G: Html,
{
    let value_str = create_signal(cx, props.value.to_string());

    create_effect(cx, move || {
        match value_str
            .get()
            .parse::<T>()
            .map_err(|e| e.to_string())
            .and_then(|x| {
                if (props.min..=props.max).contains(&x) {
                    Ok(x)
                } else {
                    Err("Out of bounds".to_owned())
                }
            }) {
            Ok(x) => {
                props.valid.set(true);
                props.value.set_silent(x);
            }
            Err(_) => {
                props.valid.set(false);
            }
        }
    });
    create_effect(cx, || {
        value_str.set(props.value.get().to_string());
    });

    view! { cx,
        div(class="setting") {
            label(for=props.id) { (props.label) }
            input(type="text", size=10, id=props.id, name=props.id, bind:value=value_str) {}
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
        div(class="setting checkbox_setting") {
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
    let curr_directory_watch = create_signal(cx, false);
    let curr_directory_empty = create_memo(cx, || curr_directory.get().path.as_os_str().is_empty());

    create_effect(cx, || {
        curr_directory.modify().exclude = curr_directory_exclude_str.get().parse().unwrap();
    });
    create_effect(cx, || {
        curr_directory.modify().watch = *curr_directory_watch.get();
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
        let mut curr_dir = std::mem::take(curr_directory.modify().deref_mut());
        curr_dir.watch &= !curr_dir.exclude;
        directory_list.modify().push(DirectoryItem::new(curr_dir));
        curr_directory_exclude_str.set(curr_directory.get().exclude.to_string());
        curr_directory_watch.set(curr_directory.get().watch);
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
                        p { (if item.dir.watch { "Отслеживается" } else { "Не отслеживается" }) }
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
            input(type="checkbox", id="curr_directory_watch", name="curr_directory_watch",
                disabled=*curr_directory_exclude_str.get() == "true", bind:checked=curr_directory_watch)
            label(for="curr_directory_watch") { "Отслеживать" }
            button(type="button", on:click=add_item, disabled=*curr_directory_empty.get()) { "➕" }
        }
    }
}
