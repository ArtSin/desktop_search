use std::{fmt::Display, str::FromStr};

use chrono::{DateTime, Local, TimeZone, Utc};
use sycamore::prelude::*;

pub mod content_type;

#[derive(Prop)]
pub struct RadioFilterProps<'a, T: Copy> {
    pub text: &'static str,
    pub name: &'static str,
    pub id: &'static str,
    pub value_signal: &'a Signal<T>,
    pub value: T,
    pub default: bool,
}

#[component]
pub fn RadioFilter<'a, T: Copy, G: Html>(cx: Scope<'a>, props: RadioFilterProps<'a, T>) -> View<G> {
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
pub struct CheckboxFilterProps<'a> {
    pub text: &'static str,
    pub id: &'static str,
    pub value_enabled: &'a Signal<bool>,
}

#[component]
pub fn CheckboxFilter<'a, G: Html>(cx: Scope<'a>, props: CheckboxFilterProps<'a>) -> View<G> {
    view! { cx,
        div(class="radio_checkbox_field") {
            input(type="checkbox", id=props.id, name=props.id, bind:checked=props.value_enabled) {}
            label(for=props.id) { (props.text) }
        }
    }
}

#[derive(Prop)]
pub struct DateTimeFilterProps<'a> {
    pub legend: &'static str,
    pub id: &'static str,
    pub value_from: &'a Signal<Option<DateTime<Utc>>>,
    pub value_to: &'a Signal<Option<DateTime<Utc>>>,
    pub valid: &'a Signal<bool>,
}

#[component]
pub fn DateTimeFilter<'a, G: Html>(cx: Scope<'a>, props: DateTimeFilterProps<'a>) -> View<G> {
    const FORMAT_STR: &str = "%FT%R";

    let curr_datetime_str = format!("{}", Local::now().format(FORMAT_STR));
    let value_from = create_signal(cx, curr_datetime_str.clone());
    let value_to = create_signal(cx, curr_datetime_str);

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
                value_datetime.set(x);
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
    create_effect(cx, || props.valid.set(*valid_from.get() && *valid_to.get()));

    view! { cx,
        fieldset {
            legend { (props.legend) }
            div(class="filter_field") {
                input(type="checkbox", id=(props.id.to_owned() + "_from"),
                    name=(props.id.to_owned() + "_from"), bind:checked=enabled_from) {}
                label(for=(props.id.to_owned() + "_from")) { "От: " }
                input(type="datetime-local", disabled=!*enabled_from.get(), bind:value=value_from) {}
                (if *valid_from.get() { "✅" } else { "❌" })
            }
            div(class="filter_field") {
                input(type="checkbox", id=(props.id.to_owned() + "_to"),
                    name=(props.id.to_owned() + "_to"), bind:checked=enabled_to) {}
                label(for=(props.id.to_owned() + "_to")) { "До: " }
                input(type="datetime-local", disabled=!*enabled_to.get(), bind:value=value_to) {}
                (if *valid_to.get() { "✅" } else { "❌" })
            }
        }
    }
}

#[derive(Prop)]
pub struct NumberFilterProps<'a, T> {
    pub legend: &'static str,
    pub id: &'static str,
    pub min: T,
    pub max: T,
    pub value_from: &'a Signal<Option<T>>,
    pub value_to: &'a Signal<Option<T>>,
    pub valid: &'a Signal<bool>,
}

#[component]
pub fn NumberFilter<'a, T, G>(cx: Scope<'a>, props: NumberFilterProps<'a, T>) -> View<G>
where
    T: Copy + FromStr + Display + PartialOrd,
    <T as FromStr>::Err: Display,
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
                value_num.set(x);
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
    create_effect(cx, || props.valid.set(*valid_from.get() && *valid_to.get()));

    view! { cx,
        fieldset {
            legend { (props.legend) }
            div(class="filter_field") {
                input(type="checkbox", id=(props.id.to_owned() + "_from"),
                    name=(props.id.to_owned() + "_from"), bind:checked=enabled_from) {}
                label(for=(props.id.to_owned() + "_from")) { "От: " }
                input(type="text", size=10, disabled=!*enabled_from.get(), bind:value=value_from) {}
                (if *valid_from.get() { "✅" } else { "❌" })
            }
            div(class="filter_field") {
                input(type="checkbox", id=(props.id.to_owned() + "_to"),
                    name=(props.id.to_owned() + "_to"), bind:checked=enabled_to) {}
                label(for=(props.id.to_owned() + "_to")) { "До: " }
                input(type="text", size=10, disabled=!*enabled_to.get(), bind:value=value_to) {}
                (if *valid_to.get() { "✅" } else { "❌" })
            }
        }
    }
}
