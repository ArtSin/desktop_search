use std::{
    cmp::Eq,
    fmt::{Debug, Display},
    hash::Hash,
    str::FromStr,
};

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
pub struct CheckboxOptionFilterProps<'a> {
    pub text: &'static str,
    pub id: &'static str,
    pub value_enabled: &'a Signal<Option<bool>>,
}

#[component]
pub fn CheckboxOptionFilter<'a, G: Html>(
    cx: Scope<'a>,
    props: CheckboxOptionFilterProps<'a>,
) -> View<G> {
    let enabled = create_signal(cx, false);
    let value = create_signal(cx, false);

    create_effect(cx, || {
        props.value_enabled.set(enabled.get().then(|| *value.get()));
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
pub struct SelectFilterProps<'a, T> {
    pub text: &'static str,
    pub id: &'static str,
    pub options: &'a ReadSignal<Vec<(T, &'static str)>>,
    pub value: &'a Signal<T>,
}

#[component]
pub fn SelectFilter<'a, T, G>(cx: Scope<'a>, props: SelectFilterProps<'a, T>) -> View<G>
where
    T: Copy + Eq + Hash + Display + FromStr,
    <T as FromStr>::Err: Debug,
    G: Html,
{
    let value_str = create_signal(cx, props.options.get().first().unwrap().0.to_string());
    create_effect(cx, || props.value.set(value_str.get().parse().unwrap()));

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
pub struct SelectOptionFilterProps<'a, T> {
    pub text: &'static str,
    pub id: &'static str,
    pub options: &'a ReadSignal<Vec<(T, &'static str)>>,
    pub value: &'a Signal<Option<T>>,
}

#[component]
pub fn SelectOptionFilter<'a, T, G>(cx: Scope<'a>, props: SelectOptionFilterProps<'a, T>) -> View<G>
where
    T: Copy + Eq + Hash + Display + FromStr,
    <T as FromStr>::Err: Debug,
    G: Html,
{
    let enabled = create_signal(cx, false);
    let value_str = create_signal(cx, props.options.get().first().unwrap().0.to_string());
    create_effect(cx, || {
        props
            .value
            .set(enabled.get().then(|| value_str.get().parse().unwrap()))
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

#[derive(Prop)]
pub struct RangeWidgetProps<'a, T> {
    pub legend: &'static str,
    pub id: &'static str,
    pub min: T,
    pub max: T,
    pub step: T,
    pub value: &'a Signal<T>,
}

#[component]
pub fn RangeWidget<'a, T, G>(cx: Scope<'a>, props: RangeWidgetProps<'a, T>) -> View<G>
where
    T: FromStr + Display + 'static,
    <T as FromStr>::Err: std::fmt::Debug,
    G: Html,
{
    let value_str = create_signal(cx, props.value.get().to_string());
    create_effect(cx, || props.value.set(value_str.get().parse().unwrap()));

    view! { cx,
        fieldset {
            legend { (props.legend) }
            div(class="filter_field") {
                label(for=props.id) { (format!("{:.1}", props.value.get())) " " }
                input(type="range", min=props.min, max=props.max, step=props.step, bind:value=value_str) {}
            }
        }
    }
}
