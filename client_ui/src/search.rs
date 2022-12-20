use std::path::PathBuf;

use chrono::{DateTime, Local, TimeZone, Utc};
use common_lib::{
    elasticsearch::FileES,
    search::{ImageSearchRequest, SearchRequest, SearchResponse, TextSearchRequest},
};
use serde::Serialize;
use serde_wasm_bindgen::{from_value, to_value};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use url::Url;

use crate::{
    app::{invoke, StatusMessage},
    settings::{MAX_FILE_SIZE_MAX, MAX_FILE_SIZE_MIN},
};

#[derive(Debug, Clone, Copy)]
enum QueryType {
    Text,
    Image,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchRequestArgs<'a> {
    search_request: &'a SearchRequest,
}

#[component]
pub fn Search<G: Html>(cx: Scope) -> View<G> {
    let status_str = create_signal(cx, String::new());

    let query = create_signal(cx, String::new());
    let query_image_path = create_signal(cx, PathBuf::new());

    let query_type = create_signal(cx, QueryType::Text);
    let image_search_enabled = create_signal(cx, true);

    let modified_from = create_signal(cx, None);
    let modified_to = create_signal(cx, None);
    let modified_valid = create_signal(cx, true);
    let size_from = create_signal(cx, None);
    let size_to = create_signal(cx, None);
    let size_valid = create_signal(cx, true);

    let any_invalid = create_memo(cx, || !*modified_valid.get() || !*size_valid.get());

    let search_results = create_signal(cx, Vec::new());

    let select_file = move |_| {
        spawn_local_scoped(cx, async {
            let path_option: Option<PathBuf> = from_value(
                invoke("pick_file", wasm_bindgen::JsValue::UNDEFINED)
                    .await
                    .unwrap(),
            )
            .unwrap();

            if let Some(path) = path_option {
                query_image_path.set(path);
            }
        });
    };

    let search = move |_| {
        spawn_local_scoped(cx, async move {
            status_str.set("⏳ Загрузка...".to_owned());

            let search_query = match *query_type.get() {
                QueryType::Text => common_lib::search::QueryType::Text(TextSearchRequest {
                    query: (*query.get()).clone(),
                    image_search_enabled: *image_search_enabled.get(),
                }),
                QueryType::Image => common_lib::search::QueryType::Image(ImageSearchRequest {
                    image_path: (*query_image_path.get()).clone(),
                }),
            };
            let search_request = SearchRequest {
                query: search_query,
                modified_from: *modified_from.get(),
                modified_to: *modified_to.get(),
                size_from: size_from.get().map(|x| (x * 1024.0 * 1024.0) as u64),
                size_to: size_to.get().map(|x| (x * 1024.0 * 1024.0) as u64),
            };

            match invoke(
                "search",
                to_value(&SearchRequestArgs {
                    search_request: &search_request,
                })
                .unwrap(),
            )
            .await
            .map_err(|e| e.as_string().unwrap())
            .and_then(|x| from_value::<SearchResponse>(x).map_err(|e| e.to_string()))
            {
                Ok(x) => {
                    search_results.set(x.results);
                    status_str.set("".to_owned());
                }
                Err(e) => {
                    search_results.set(Vec::new());
                    status_str.set("❌ Ошибка поиска: ".to_owned() + &e);
                }
            }
        })
    };

    view! { cx,
        header {
            (match *query_type.get() {
                QueryType::Text => {
                    view! { cx,
                        div {
                            input(form="search", type="search", id="query", name="query",
                                placeholder="Поиск...", bind:value=query)
                            button(form="search", type="submit", disabled=*any_invalid.get()) { "Искать" }
                        }
                    }
                }
                QueryType::Image => {
                    view! { cx,
                        div {
                            button(form="search", type="button", on:click=select_file) { "Выбрать файл" }
                            button(form="search", type="submit", disabled=*any_invalid.get()) { "Искать" }
                        }
                        (if !query_image_path.get().as_os_str().is_empty() {
                            let path = query_image_path.get().to_string_lossy().into_owned();
                            let img_url = Url::parse(&("localfile://localhost".to_owned() + &path)).unwrap();
                            view! { cx,
                                div {
                                    img(src=img_url, id="query_image") {}
                                }
                            }
                        } else {
                            view! { cx, }
                        })
                    }
                }
            })
        }
        div(class="main_container") {
            aside {
                form(id="search", on:submit=search, action="javascript:void(0);") {
                    fieldset {
                        legend { "Тип запроса" }
                        RadioFilter(text="По тексту", name="query_type", id="query_type_text",
                            value_signal=query_type, value=QueryType::Text, default=true)
                        RadioFilter(text="По изображению", name="query_type", id="query_type_image",
                            value_signal=query_type, value=QueryType::Image, default=false)
                    }
                    (match *query_type.get() {
                        QueryType::Text => {
                            view! { cx,
                                fieldset {
                                    legend { "Тип поиска" }
                                    CheckboxFilter(text="Семантический поиск по изображениям", id="image_search",
                                        value_enabled=image_search_enabled)
                                }
                            }
                        }
                        QueryType::Image => {
                            view! { cx, }
                        }
                    })

                    DateTimeFilter(legend="Дата и время изменения", id="modified",
                        value_from=modified_from, value_to=modified_to, valid=modified_valid)

                    NumberFilter(legend="Размер файла (МиБ)", id="size",
                        min=MAX_FILE_SIZE_MIN, max=MAX_FILE_SIZE_MAX,
                        value_from=size_from, value_to=size_to, valid=size_valid)
                }
            }
            main {
                StatusMessage(status_str=status_str)
                SearchResults(search_results=search_results)
            }
        }
    }
}

#[derive(Prop)]
struct RadioFilterProps<'a, T: Copy> {
    text: &'static str,
    name: &'static str,
    id: &'static str,
    value_signal: &'a Signal<T>,
    value: T,
    default: bool,
}

#[component]
fn RadioFilter<'a, T: Copy, G: Html>(cx: Scope<'a>, props: RadioFilterProps<'a, T>) -> View<G> {
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
struct CheckboxFilterProps<'a> {
    text: &'static str,
    id: &'static str,
    value_enabled: &'a Signal<bool>,
}

#[component]
fn CheckboxFilter<'a, G: Html>(cx: Scope<'a>, props: CheckboxFilterProps<'a>) -> View<G> {
    view! { cx,
        div(class="radio_checkbox_field") {
            input(type="checkbox", id=props.id, name=props.id, bind:checked=props.value_enabled) {}
            label(for=props.id) { (props.text) }
        }
    }
}

#[derive(Prop)]
struct DateTimeFilterProps<'a> {
    legend: &'static str,
    id: &'static str,
    value_from: &'a Signal<Option<DateTime<Utc>>>,
    value_to: &'a Signal<Option<DateTime<Utc>>>,
    valid: &'a Signal<bool>,
}

#[component]
fn DateTimeFilter<'a, G: Html>(cx: Scope<'a>, props: DateTimeFilterProps<'a>) -> View<G> {
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
struct NumberFilterProps<'a> {
    legend: &'static str,
    id: &'static str,
    min: f64,
    max: f64,
    value_from: &'a Signal<Option<f64>>,
    value_to: &'a Signal<Option<f64>>,
    valid: &'a Signal<bool>,
}

#[component]
fn NumberFilter<'a, G: Html>(cx: Scope<'a>, props: NumberFilterProps<'a>) -> View<G> {
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
            value
                .parse::<f64>()
                .map_err(|e| e.to_string())
                .and_then(|x| {
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
                       value_num: &Signal<Option<f64>>| {
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

#[component(inline_props)]
fn SearchResults<'a, G: Html>(
    cx: Scope<'a>,
    search_results: &'a ReadSignal<Vec<FileES>>,
) -> View<G> {
    view! { cx,
        Keyed(
            iterable=search_results,
            view=move |cx, item| {
                let file_name = item.path.file_name().unwrap().to_string_lossy().into_owned();
                let path = item.path.to_string_lossy().into_owned();
                let path_ = path.clone();

                view! { cx,
                    article(class="search_result") {
                        (if item.content_type.starts_with("image") {
                            let img_url = Url::parse(&("localfile://localhost".to_owned() + &path_)).unwrap();
                            view! { cx,
                                img(src=(img_url)) {}
                            }
                        } else {
                            view! { cx, }
                        })

                        h3 {
                            (file_name)
                        }
                        p(style="overflow-wrap: anywhere;") {
                            "Полный путь: " (path)
                        }
                        p {
                            "Изменено: " (item.modified.with_timezone(&Local))
                        }
                        p {
                            "Размер (МиБ): "
                            (format!("{:.4}", (item.size as f64) / 1024.0 / 1024.0))
                        }
                        p(style="overflow-wrap: anywhere;") {
                            "Хеш SHA-256: " (item.hash)
                        }
                    }
                }
            },
            key=|item| item._id.clone().unwrap(),
        )
    }
}
