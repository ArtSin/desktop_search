use std::{
    cmp::Eq,
    fmt::{Debug, Display},
    hash::Hash,
    path::PathBuf,
    str::FromStr,
};

use chrono::{DateTime, Local, TimeZone, Utc};
use common_lib::actions::PickFolderResult;
use fluent_bundle::FluentArgs;
use sycamore::{futures::spawn_local_scoped, prelude::*};
use wasm_bindgen::JsValue;

use crate::app::{fetch, get_translation, widgets::StatusDialogState};

pub mod content_type;

#[derive(Prop)]
pub struct RadioFilterProps<'a, T: Copy, S: AsRef<str>> {
    pub text: S,
    pub name: &'static str,
    pub id: &'static str,
    pub value_signal: &'a Signal<T>,
    pub value: T,
    pub default: bool,
}

#[component]
pub fn RadioFilter<'a, T, S, G>(cx: Scope<'a>, props: RadioFilterProps<'a, T, S>) -> View<G>
where
    T: Copy,
    S: 'static + AsRef<str> + Display,
    G: Html,
{
    let update = move |_| {
        props.value_signal.set(props.value);
    };
    view! { cx,
        div(class="radio_checkbox_field") {
            input(type="radio", id=props.id, name=props.name, value=props.id,
                on:change=update, checked=props.default) {}
            label(for=props.id) { (props.text) }
        }
    }
}

#[derive(Prop)]
pub struct CheckboxFilterProps<'a, S: AsRef<str>> {
    pub text: S,
    pub id: &'static str,
    pub value_enabled: &'a Signal<bool>,
}

#[component]
pub fn CheckboxFilter<'a, S: 'static + AsRef<str> + Display, G: Html>(
    cx: Scope<'a>,
    props: CheckboxFilterProps<'a, S>,
) -> View<G> {
    view! { cx,
        div(class="radio_checkbox_field") {
            input(type="checkbox", id=props.id, name=props.id, bind:checked=props.value_enabled) {}
            label(for=props.id) { (props.text) }
        }
    }
}

#[derive(Prop)]
pub struct CheckboxOptionFilterProps<'a, S: AsRef<str>> {
    pub text: S,
    pub id: &'static str,
    pub value_enabled: &'a Signal<Option<bool>>,
}

#[component]
pub fn CheckboxOptionFilter<'a, S: 'static + AsRef<str> + Display, G: Html>(
    cx: Scope<'a>,
    props: CheckboxOptionFilterProps<'a, S>,
) -> View<G> {
    let enabled = create_signal(cx, false);
    let value = create_signal(cx, false);

    create_effect(cx, || {
        props
            .value_enabled
            .set_silent(enabled.get().then(|| *value.get()));
    });
    create_effect(cx, || {
        let val = props.value_enabled.get();
        enabled.set(val.is_some());
        value.set(val.unwrap_or_default());
    });

    view! { cx,
        div(class="radio_checkbox_field") {
            input(type="checkbox", id=(props.id.to_owned() + "_enabled"),
                    name=(props.id.to_owned() + "_enabled"), bind:checked=enabled)
            label(for=(props.id.to_owned() +  "_enabled")) { (props.text) }
            input(type="checkbox", id=(props.id.to_owned() + "_value"),
                    name=(props.id.to_owned() + "_value"), disabled=!*enabled.get(), bind:checked=value)
        }
    }
}

#[derive(Prop)]
pub struct SelectFilterProps<'a, T, S: AsRef<str>> {
    pub text: S,
    pub id: &'static str,
    pub options: &'a ReadSignal<Vec<(T, S)>>,
    pub value: &'a Signal<T>,
}

#[component]
pub fn SelectFilter<'a, T, S, G>(cx: Scope<'a>, props: SelectFilterProps<'a, T, S>) -> View<G>
where
    T: Copy + Eq + Hash + Display + FromStr,
    <T as FromStr>::Err: Debug,
    S: 'static + AsRef<str> + Clone + Display + PartialEq,
    G: Html,
{
    let value_str = create_signal(cx, props.options.get().first().unwrap().0.to_string());
    create_effect(cx, || {
        props.value.set_silent(value_str.get().parse().unwrap())
    });
    create_effect(cx, || value_str.set(props.value.get().to_string()));

    view! { cx,
        div(class="filter_field") {
            label(for=props.id) { (props.text) }
            select(id=props.id, name=props.id, bind:value=value_str) {
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

#[derive(Prop)]
pub struct SelectOptionFilterProps<'a, T, S: AsRef<str>> {
    pub text: S,
    pub id: &'static str,
    pub options: &'a ReadSignal<Vec<(T, S)>>,
    pub value: &'a Signal<Option<T>>,
}

#[component]
pub fn SelectOptionFilter<'a, T, S, G>(
    cx: Scope<'a>,
    props: SelectOptionFilterProps<'a, T, S>,
) -> View<G>
where
    T: Copy + Eq + Hash + Display + FromStr,
    <T as FromStr>::Err: Debug,
    S: 'static + AsRef<str> + Clone + Display + PartialEq,
    G: Html,
{
    let enabled = create_signal(cx, false);
    let value_str = create_signal(cx, props.options.get().first().unwrap().0.to_string());
    create_effect(cx, || {
        props
            .value
            .set_silent(enabled.get().then(|| value_str.get().parse().unwrap()))
    });
    create_effect(cx, || {
        let val = props.value.get();
        enabled.set(val.is_some());
        value_str.set(
            val.map(|x| x.to_string())
                .unwrap_or_else(|| props.options.get().first().unwrap().0.to_string()),
        );
    });

    view! { cx,
        div(class="filter_field") {
            input(type="checkbox", id=(props.id.to_owned() + "_enabled"),
                    name=(props.id.to_owned() + "_enabled"), bind:checked=enabled)
            label(for=(props.id.to_owned() + "_enabled")) { (props.text) }
            select(id=(props.id.to_owned() + "_value"), name=(props.id.to_owned() + "_value"),
                    disabled=!*enabled.get(), bind:value=value_str) {
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

#[derive(Prop)]
pub struct DateTimeFilterProps<'a, S: AsRef<str>> {
    pub legend: S,
    pub id: &'static str,
    pub value_from: &'a Signal<Option<DateTime<Utc>>>,
    pub value_to: &'a Signal<Option<DateTime<Utc>>>,
    pub valid: &'a Signal<bool>,
}

#[component]
pub fn DateTimeFilter<'a, S: 'static + AsRef<str> + Display, G: Html>(
    cx: Scope<'a>,
    props: DateTimeFilterProps<'a, S>,
) -> View<G> {
    const FORMAT_STR: &str = "%FT%R";

    let curr_datetime_str = || format!("{}", Local::now().format(FORMAT_STR));
    let value_from = create_signal(cx, curr_datetime_str());
    let value_to = create_signal(cx, curr_datetime_str());

    let enabled_from = create_signal(cx, false);
    let enabled_to = create_signal(cx, false);

    let valid_from = create_signal(cx, true);
    let valid_to = create_signal(cx, true);

    let parse = |enabled: bool, value: &str| {
        if !enabled {
            Ok(None)
        } else {
            Local
                .datetime_from_str(value, FORMAT_STR)
                .map(|x| Some(DateTime::from(x)))
        }
    };

    let update = move |enabled: &Signal<bool>,
                       value_str: &Signal<String>,
                       valid: &Signal<bool>,
                       value_datetime: &Signal<Option<DateTime<Utc>>>| {
        match parse(*enabled.get(), &value_str.get()) {
            Ok(x) => {
                valid.set(true);
                value_datetime.set_silent(x);
            }
            Err(_) => {
                valid.set(false);
            }
        }
    };
    create_effect(cx, move || {
        update(enabled_from, value_from, valid_from, props.value_from);
        update(enabled_to, value_to, valid_to, props.value_to);
    });
    create_effect(cx, move || {
        let value_from_date = props.value_from.get();
        enabled_from.set(value_from_date.is_some());
        value_from.set(
            value_from_date
                .map(|x| format!("{}", x.format(FORMAT_STR)))
                .unwrap_or_else(curr_datetime_str),
        );
    });
    create_effect(cx, move || {
        let value_to_date = props.value_to.get();
        enabled_to.set(value_to_date.is_some());
        value_to.set(
            value_to_date
                .map(|x| format!("{}", x.format(FORMAT_STR)))
                .unwrap_or_else(curr_datetime_str),
        );
    });
    create_effect(cx, || props.valid.set(*valid_from.get() && *valid_to.get()));

    view! { cx,
        fieldset {
            legend { (props.legend) }
            div(class="filter_field") {
                input(type="checkbox", id=(props.id.to_owned() + "_from"),
                    name=(props.id.to_owned() + "_from"), bind:checked=enabled_from) {}
                label(for=(props.id.to_owned() + "_from")) { (get_translation("filter_from", None)) }
                input(type="datetime-local", disabled=!*enabled_from.get(), bind:value=value_from) {}
                (if *valid_from.get() { "✅" } else { "❌" })
            }
            div(class="filter_field") {
                input(type="checkbox", id=(props.id.to_owned() + "_to"),
                    name=(props.id.to_owned() + "_to"), bind:checked=enabled_to) {}
                label(for=(props.id.to_owned() + "_to")) { (get_translation("filter_to", None)) }
                input(type="datetime-local", disabled=!*enabled_to.get(), bind:value=value_to) {}
                (if *valid_to.get() { "✅" } else { "❌" })
            }
        }
    }
}

#[derive(Prop)]
pub struct NumberFilterProps<'a, T, S: AsRef<str>> {
    pub legend: S,
    pub id: &'static str,
    pub min: T,
    pub max: T,
    pub value_from: &'a Signal<Option<T>>,
    pub value_to: &'a Signal<Option<T>>,
    pub valid: &'a Signal<bool>,
}

#[component]
pub fn NumberFilter<'a, T, S, G>(cx: Scope<'a>, props: NumberFilterProps<'a, T, S>) -> View<G>
where
    T: Copy + FromStr + Display + PartialOrd,
    <T as FromStr>::Err: Display,
    S: 'static + AsRef<str> + Display,
    G: Html,
{
    let value_from = create_signal(cx, props.min.to_string());
    let value_to = create_signal(cx, props.max.to_string());

    let enabled_from = create_signal(cx, false);
    let enabled_to = create_signal(cx, false);

    let valid_from = create_signal(cx, true);
    let valid_to = create_signal(cx, true);

    let parse = move |enabled: bool, value: &str| {
        if !enabled {
            Ok(None)
        } else {
            value.parse::<T>().map_err(|e| e.to_string()).and_then(|x| {
                if (props.min..=props.max).contains(&x) {
                    Ok(Some(x))
                } else {
                    Err("Out of bounds".to_owned())
                }
            })
        }
    };
    let update = move |enabled: &Signal<bool>,
                       value_str: &Signal<String>,
                       valid: &Signal<bool>,
                       value_num: &Signal<Option<T>>| {
        match parse(*enabled.get(), &value_str.get()) {
            Ok(x) => {
                valid.set(true);
                value_num.set_silent(x);
            }
            Err(_) => {
                valid.set(false);
            }
        }
    };
    create_effect(cx, move || {
        update(enabled_from, value_from, valid_from, props.value_from);
        update(enabled_to, value_to, valid_to, props.value_to);
    });
    create_effect(cx, move || {
        let value_from_num = props.value_from.get();
        enabled_from.set(value_from_num.is_some());
        value_from.set(
            value_from_num
                .map(|x| x.to_string())
                .unwrap_or_else(|| props.min.to_string()),
        );
    });
    create_effect(cx, move || {
        let value_to_num = props.value_to.get();
        enabled_to.set(value_to_num.is_some());
        value_to.set(
            value_to_num
                .map(|x| x.to_string())
                .unwrap_or_else(|| props.max.to_string()),
        );
    });
    create_effect(cx, || props.valid.set(*valid_from.get() && *valid_to.get()));

    view! { cx,
        fieldset {
            legend { (props.legend) }
            div(class="filter_field") {
                input(type="checkbox", id=(props.id.to_owned() + "_from"),
                    name=(props.id.to_owned() + "_from"), bind:checked=enabled_from) {}
                label(for=(props.id.to_owned() + "_from")) { (get_translation("filter_from", None)) }
                input(type="text", size=10, disabled=!*enabled_from.get(), bind:value=value_from) {}
                (if *valid_from.get() { "✅" } else { "❌" })
            }
            div(class="filter_field") {
                input(type="checkbox", id=(props.id.to_owned() + "_to"),
                    name=(props.id.to_owned() + "_to"), bind:checked=enabled_to) {}
                label(for=(props.id.to_owned() + "_to")) { (get_translation("filter_to", None)) }
                input(type="text", size=10, disabled=!*enabled_to.get(), bind:value=value_to) {}
                (if *valid_to.get() { "✅" } else { "❌" })
            }
        }
    }
}

#[derive(Prop)]
pub struct RangeWidgetProps<'a, T, S: AsRef<str>> {
    pub legend: S,
    pub id: &'static str,
    pub min: T,
    pub max: T,
    pub step: T,
    pub value: &'a Signal<T>,
}

#[component]
pub fn RangeWidget<'a, T, S, G>(cx: Scope<'a>, props: RangeWidgetProps<'a, T, S>) -> View<G>
where
    T: 'static + FromStr + Display,
    <T as FromStr>::Err: std::fmt::Debug,
    S: 'static + AsRef<str> + Display,
    G: Html,
{
    let value_str = create_signal(cx, props.value.get().to_string());
    let value_formatted = create_signal(cx, format!("{:.1}", props.value.get()));
    create_effect(cx, || {
        let val = value_str.get().parse().unwrap();
        value_formatted.set(format!("{:.1}", val));
        props.value.set_silent(val);
    });
    create_effect(cx, || value_str.set(props.value.get().to_string()));

    view! { cx,
        fieldset {
            legend { (props.legend) }
            div(class="filter_field") {
                label(for=props.id) { (value_formatted.get()) " " }
                input(type="range", id=props.id, min=props.min, max=props.max, step=props.step, bind:value=value_str) {}
            }
        }
    }
}

async fn pick_folder() -> Result<PickFolderResult, JsValue> {
    fetch("/pick_folder", "POST", None::<&()>).await
}

#[derive(Prop)]
pub struct PathFilterProps<'a, S: AsRef<str>> {
    pub legend: S,
    pub id: &'static str,
    pub value: &'a Signal<Option<PathBuf>>,
    pub status_dialog_state: &'a Signal<StatusDialogState>,
}

#[component]
pub fn PathFilter<'a, S: 'static + AsRef<str> + Display, G: Html>(
    cx: Scope<'a>,
    props: PathFilterProps<'a, S>,
) -> View<G> {
    let enabled = create_signal(cx, false);
    let value = create_signal(cx, PathBuf::new());
    let value_str = create_memo(cx, || value.get().to_string_lossy().into_owned());

    create_effect(cx, || {
        props
            .value
            .set_silent(enabled.get().then(|| value.get().as_ref().clone()))
    });
    create_effect(cx, || {
        value.set((*props.value.get()).clone().unwrap_or_default())
    });

    let select_directory = move |_| {
        spawn_local_scoped(cx, async {
            match pick_folder().await {
                Ok(res) => {
                    if let Some(path) = res.path {
                        *value.modify() = path;
                    }
                }
                Err(e) => {
                    let error_args = FluentArgs::from_iter([("error", format!("{e:#?}"))]);
                    let error_str =
                        get_translation("dialog_opening_error", Some(&error_args)).to_string();
                    props
                        .status_dialog_state
                        .set(StatusDialogState::Error(error_str));
                }
            }
        });
    };

    view! { cx,
        fieldset {
            legend { (props.legend) }
            div(class="filter_field") {
                input(type="checkbox", id=props.id, name=props.id, bind:checked=enabled)
                input(type="text", size=7, disabled=!*enabled.get(), readonly=true, value=value_str)
                button(type="button", on:click=select_directory) { (get_translation("select", None)) }
            }
        }
    }
}
