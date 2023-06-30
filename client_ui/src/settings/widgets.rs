use std::{
    fmt::{Debug, Display},
    hash::Hash,
    ops::DerefMut,
    str::FromStr,
};

use common_lib::{
    actions::PickFolderResult,
    settings::{IndexingDirectory, NNDevice, NNSettings},
};
use fluent_bundle::{FluentArgs, FluentValue};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use uuid::Uuid;
use wasm_bindgen::JsValue;

use crate::{
    app::{fetch, get_translation, widgets::StatusDialogState},
    settings::{BATCH_SIZE_MAX, BATCH_SIZE_MIN, MAX_DELAY_MS_MAX, MAX_DELAY_MS_MIN},
};

#[derive(Prop)]
pub struct SimpleTextSettingProps<'a, S: AsRef<str>> {
    pub id: &'static str,
    pub label: S,
    pub value: &'a Signal<String>,
}

#[component]
pub fn SimpleTextSetting<'a, S: 'static + AsRef<str> + Display, G: Html>(
    cx: Scope<'a>,
    props: SimpleTextSettingProps<'a, S>,
) -> View<G> {
    let value = props.value;
    view! { cx,
        div(class="setting") {
            label(for=props.id) { (props.label) }
            input(type="text", id=props.id, name=props.id, bind:value=value) {}
        }
    }
}

#[derive(Prop)]
pub struct TextSettingProps<'a, T, S: AsRef<str>, F> {
    pub id: &'static str,
    pub label: S,
    pub parse: F,
    pub value: &'a Signal<T>,
    pub valid: &'a Signal<bool>,
}

#[component]
pub fn TextSetting<'a, T, S, E, F, G>(
    cx: Scope<'a>,
    props: TextSettingProps<'a, T, S, F>,
) -> View<G>
where
    T: Display,
    S: 'static + AsRef<str> + Display,
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
pub struct NumberSettingProps<'a, T, S: AsRef<str>> {
    pub id: String,
    pub label: S,
    pub min: T,
    pub max: T,
    pub value: &'a Signal<T>,
    pub valid: &'a Signal<bool>,
}

#[component]
pub fn NumberSetting<'a, T, S, G>(cx: Scope<'a>, props: NumberSettingProps<'a, T, S>) -> View<G>
where
    T: Copy + FromStr + Display + PartialOrd,
    <T as FromStr>::Err: Display,
    S: 'static + AsRef<str> + Display,
    G: Html,
{
    let id = props.id.clone();
    let id_ = props.id.clone();

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
            input(type="text", size=10, id=id, name=id_, bind:value=value_str) {}
            (if *props.valid.get() { "✅" } else { "❌" })
        }
    }
}

#[derive(Prop)]
pub struct CheckboxSettingProps<'a, S> {
    pub id: &'static str,
    pub label: S,
    pub value: &'a Signal<bool>,
}

#[component]
pub fn CheckboxSetting<'a, S: 'static + AsRef<str> + Display, G: Html>(
    cx: Scope<'a>,
    props: CheckboxSettingProps<'a, S>,
) -> View<G> {
    let value = props.value;
    view! { cx,
        div(class="setting checkbox_setting") {
            label(for=props.id) { (props.label) }
            input(type="checkbox", id=props.id, name=props.id, bind:checked=value) {}
        }
    }
}

#[derive(Prop)]
pub struct SelectSettingProps<'a, T, S: AsRef<str>> {
    pub id: String,
    pub label: String,
    pub options: &'a ReadSignal<Vec<(T, S)>>,
    pub value: &'a Signal<T>,
}

#[component]
pub fn SelectSetting<'a, T, S, G>(cx: Scope<'a>, props: SelectSettingProps<'a, T, S>) -> View<G>
where
    T: Copy + Eq + Hash + Display + FromStr,
    <T as FromStr>::Err: Debug,
    S: 'static + AsRef<str> + Clone + Display + PartialEq,
    G: Html,
{
    let id = props.id.clone();
    let id_ = props.id.clone();

    let value_str = create_signal(cx, props.value.get().to_string());
    create_effect(cx, || {
        props.value.set_silent(value_str.get().parse().unwrap());
    });
    create_effect(cx, || value_str.set(props.value.get().to_string()));

    view! { cx,
        div(class="setting checkbox_setting") {
            label(for=props.id) { (props.label) }
            select(id=id, name=id_, bind:value=value_str) {
                Keyed(
                    iterable=props.options,
                    key=|item| item.0,
                    view=move |cx, item| {
                        view! { cx,
                            option(value=(item.0)) { (item.1) }
                        }
                    }
                )
            }
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
                    let error_args = FluentArgs::from_iter([("error", format!("{e:#?}"))]);
                    let error_str =
                        get_translation("dialog_opening_error", Some(&error_args)).to_string();
                    status_dialog_state.set(StatusDialogState::Error(error_str));
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
                        p { (if item.dir.exclude { get_translation("excluded", None) } else { get_translation("included", None) }) }
                        p { (if item.dir.watch { get_translation("watching", None) } else { get_translation("not_watching", None) }) }
                        button(type="button", on:click=delete_item) { "➖" }
                    }
                }
            }
        )

        div(class="setting") {
            input(type="text", readonly=true, value=curr_directory.get().path.display()) {}
            button(type="button", on:click=select_item) { (get_translation("select", None)) }
            select(bind:value=curr_directory_exclude_str) {
                option(selected=true, value="false") { (get_translation("include", None)) }
                option(value="true") { (get_translation("exclude", None)) }
            }
            input(type="checkbox", id="curr_directory_watch", name="curr_directory_watch",
                disabled=*curr_directory_exclude_str.get() == "true", bind:checked=curr_directory_watch)
            label(for="curr_directory_watch") { (get_translation("watch", None)) }
            button(type="button", on:click=add_item, disabled=*curr_directory_empty.get()) { "➕" }
        }
    }
}

#[derive(Clone)]
pub struct NNSettingsData<'a> {
    device: &'a Signal<NNDevice>,
    batch_size: &'a Signal<usize>,
    max_delay_ms: &'a Signal<u64>,

    batch_size_valid: &'a Signal<bool>,
    max_delay_ms_valid: &'a Signal<bool>,
    pub any_invalid: &'a ReadSignal<bool>,
}

impl<'a> NNSettingsData<'a> {
    pub fn new(cx: Scope<'a>, settings: &NNSettings) -> Self {
        let batch_size_valid = create_signal(cx, true);
        let max_delay_ms_valid = create_signal(cx, true);
        let any_invalid = create_memo(cx, || {
            !*batch_size_valid.get() || !*max_delay_ms_valid.get()
        });

        Self {
            device: create_signal(cx, settings.device),
            batch_size: create_signal(cx, settings.batch_size),
            max_delay_ms: create_signal(cx, settings.max_delay_ms),
            batch_size_valid,
            max_delay_ms_valid,
            any_invalid,
        }
    }

    pub fn to_settings(&self) -> NNSettings {
        NNSettings {
            device: *self.device.get(),
            batch_size: *self.batch_size.get(),
            max_delay_ms: *self.max_delay_ms.get(),
        }
    }

    pub fn update_from_settings(&mut self, settings: NNSettings) {
        self.device.set(settings.device);
        self.batch_size.set(settings.batch_size);
        self.max_delay_ms.set(settings.max_delay_ms);
    }
}

#[component(inline_props)]
pub fn NNSetting<'a, S, G>(
    cx: Scope<'a>,
    id: &'static str,
    label: S,
    data: &'a Signal<NNSettingsData<'a>>,
) -> View<G>
where
    S: 'static + AsRef<str>,
    FluentValue<'static>: From<S>,
    G: Html,
{
    let device_options = create_signal(
        cx,
        vec![
            (NNDevice::CPU, get_translation("cpu", None)),
            (NNDevice::CUDA, get_translation("cuda", None)),
        ],
    );

    let label_args = FluentArgs::from_iter([("model", label)]);
    let label_device = get_translation("nn_setting_device", Some(&label_args)).to_string();
    let label_batch_size = get_translation("nn_setting_batch_size", Some(&label_args)).to_string();
    let label_max_delay = get_translation("nn_setting_max_delay", Some(&label_args)).to_string();

    view! { cx,
        SelectSetting(id=id.to_owned() + "_device", label=label_device,
            options=device_options, value=data.get().device)
        NumberSetting(id=id.to_owned() + "_batch_size", label=label_batch_size,
            min=BATCH_SIZE_MIN, max=BATCH_SIZE_MAX,
            value=data.get().batch_size, valid=data.get().batch_size_valid)
        NumberSetting(id=id.to_owned() + "_max_delay", label=label_max_delay,
            min=MAX_DELAY_MS_MIN, max=MAX_DELAY_MS_MAX,
            value=data.get().max_delay_ms, valid=data.get().max_delay_ms_valid)
    }
}
