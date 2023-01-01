use std::{
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::{DateTime, Local, TimeZone, Utc};
use common_lib::{
    actions::{OpenPathArgs, PickFileResult},
    elasticsearch::{FileES, FileMetadata},
    search::{
        DocumentSearchRequest, ImageQuery, ImageSearchRequest, SearchRequest, SearchResponse,
        TextQuery,
    },
};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use url::Url;
use wasm_bindgen::JsValue;

use crate::{
    app::{fetch, fetch_empty, widgets::StatusDialogState},
    settings::{MAX_FILE_SIZE_MAX, MAX_FILE_SIZE_MIN},
};

#[derive(Debug, Clone, Copy)]
enum QueryType {
    Text,
    Image,
}

#[derive(Default)]
struct PreviewData {
    path: PathBuf,
    content_type: String,
}

fn get_local_file_url<P: AsRef<Path>>(path: P, thumbnail: bool) -> Url {
    let base = Url::parse(&web_sys::window().unwrap().location().origin().unwrap()).unwrap();
    let mut file_url = base.join("/file").unwrap();
    file_url
        .query_pairs_mut()
        .append_pair("path", &path.as_ref().to_string_lossy())
        .append_pair("thumbnail", &thumbnail.to_string());
    file_url
}

async fn pick_file() -> Result<PickFileResult, JsValue> {
    fetch("/pick_file", "POST", None::<&()>).await
}

async fn search(search_request: &SearchRequest) -> Result<SearchResponse, JsValue> {
    fetch("/search", "POST", Some(search_request)).await
}

async fn open_path(args: &OpenPathArgs) -> Result<(), JsValue> {
    fetch_empty("/open_path", "POST", Some(args)).await
}

#[component(inline_props)]
pub fn Search<'a, G: Html>(
    cx: Scope<'a>,
    status_dialog_state: &'a Signal<StatusDialogState>,
) -> View<G> {
    const IMAGE_SIZE_MIN: u32 = 1;
    const IMAGE_SIZE_MAX: u32 = 99999;

    let query = create_signal(cx, String::new());
    let query_image_path = create_signal(cx, PathBuf::new());

    let query_type = create_signal(cx, QueryType::Text);
    let image_search_enabled = create_signal(cx, true);

    let display_filters = create_signal(cx, true);
    let modified_from = create_signal(cx, None);
    let modified_to = create_signal(cx, None);
    let modified_valid = create_signal(cx, true);
    let size_from = create_signal(cx, None);
    let size_to = create_signal(cx, None);
    let size_valid = create_signal(cx, true);

    let width_from = create_signal(cx, None);
    let width_to = create_signal(cx, None);
    let width_valid = create_signal(cx, true);
    let height_from = create_signal(cx, None);
    let height_to = create_signal(cx, None);
    let height_valid = create_signal(cx, true);

    let doc_created_from = create_signal(cx, None);
    let doc_created_to = create_signal(cx, None);
    let doc_created_valid = create_signal(cx, true);
    let doc_modified_from = create_signal(cx, None);
    let doc_modified_to = create_signal(cx, None);
    let doc_modified_valid = create_signal(cx, true);

    let any_invalid = create_memo(cx, || {
        !*modified_valid.get()
            || !*size_valid.get()
            || !*width_valid.get()
            || !*height_valid.get()
            || !*doc_created_valid.get()
            || !*doc_modified_valid.get()
    });

    let display_preview = create_signal(cx, false);
    let preview_data = create_signal(cx, PreviewData::default());

    let search_results = create_signal(cx, Vec::new());

    let toggle_filters = move |_| {
        display_filters.set(!*display_filters.get());
    };
    let hide_preview = move |_| {
        display_preview.set(false);
    };

    let select_file = move |_| {
        spawn_local_scoped(cx, async {
            status_dialog_state.set(StatusDialogState::Loading);

            match pick_file().await {
                Ok(res) => {
                    if let Some(path) = res.path {
                        query_image_path.set(path);
                    }
                    status_dialog_state.set(StatusDialogState::None);
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

    let search = move |_| {
        spawn_local_scoped(cx, async move {
            status_dialog_state.set(StatusDialogState::Loading);

            let search_query = match *query_type.get() {
                QueryType::Text => common_lib::search::QueryType::Text(TextQuery {
                    query: (*query.get()).clone(),
                    image_search_enabled: *image_search_enabled.get(),
                }),
                QueryType::Image => common_lib::search::QueryType::Image(ImageQuery {
                    image_path: (*query_image_path.get()).clone(),
                }),
            };
            let search_request = SearchRequest {
                query: search_query,
                modified_from: *modified_from.get(),
                modified_to: *modified_to.get(),
                size_from: size_from.get().map(|x| (x * 1024.0 * 1024.0) as u64),
                size_to: size_to.get().map(|x| (x * 1024.0 * 1024.0) as u64),
                image_data: ImageSearchRequest {
                    width_from: *width_from.get(),
                    width_to: *width_to.get(),
                    height_from: *height_from.get(),
                    height_to: *height_to.get(),
                },
                document_data: DocumentSearchRequest {
                    doc_created_from: *doc_created_from.get(),
                    doc_created_to: *doc_created_to.get(),
                    doc_modified_from: *doc_modified_from.get(),
                    doc_modified_to: *doc_modified_to.get(),
                },
            };

            match search(&search_request).await {
                Ok(x) => {
                    search_results.set(x.results);
                    status_dialog_state.set(StatusDialogState::None);
                }
                Err(e) => {
                    search_results.set(Vec::new());
                    status_dialog_state.set(StatusDialogState::Error(format!(
                        "❌ Ошибка поиска: {:#?}",
                        e
                    )));
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
                            button(form="search", type="button", on:click=toggle_filters) { "☰" }
                            input(form="search", type="search", id="query", name="query",
                                placeholder="Поиск...", bind:value=query)
                            button(form="search", type="submit", disabled=*any_invalid.get()) { "Искать" }
                        }
                    }
                }
                QueryType::Image => {
                    view! { cx,
                        div {
                            button(form="search", type="button", on:click=toggle_filters) { "☰" }
                            button(form="search", type="button", on:click=select_file) { "Выбрать файл" }
                            button(form="search", type="submit", disabled=*any_invalid.get()) { "Искать" }
                        }
                        (if !query_image_path.get().as_os_str().is_empty() {
                            let img_url = get_local_file_url(&*query_image_path.get(), false);
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
            aside(style={if *display_filters.get() { "display: block;" } else { "display: none;" }}) {
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

                    details {
                        summary { "Основные свойства файла" }

                        DateTimeFilter(legend="Дата и время изменения", id="modified",
                            value_from=modified_from, value_to=modified_to, valid=modified_valid)

                        NumberFilter(legend="Размер файла (МиБ)", id="size",
                            min=MAX_FILE_SIZE_MIN, max=MAX_FILE_SIZE_MAX,
                            value_from=size_from, value_to=size_to, valid=size_valid)
                    }

                    details {
                        summary { "Свойства изображения" }

                        NumberFilter(legend="Ширина (пиксели)", id="width",
                            min=IMAGE_SIZE_MIN, max=IMAGE_SIZE_MAX,
                            value_from=width_from, value_to=width_to, valid=width_valid)

                        NumberFilter(legend="Высота (пиксели)", id="height",
                            min=IMAGE_SIZE_MIN, max=IMAGE_SIZE_MAX,
                            value_from=height_from, value_to=height_to, valid=height_valid)
                    }

                    details {
                        summary { "Свойства документа" }

                        DateTimeFilter(legend="Дата и время создания", id="doc_created",
                            value_from=doc_created_from, value_to=doc_created_to, valid=doc_created_valid)

                        DateTimeFilter(legend="Дата и время изменения", id="doc_modified",
                            value_from=doc_modified_from, value_to=doc_modified_to, valid=doc_modified_valid)
                    }
                }
            }

            main {
                SearchResults(search_results=search_results, display_preview=display_preview,
                    preview_data=preview_data, status_dialog_state=status_dialog_state)
            }

            (if *display_preview.get() {
                let content_type = preview_data.get().content_type.clone();
                let object_url = get_local_file_url(&preview_data.get().path, false);
                view! { cx,
                    aside(id="preview") {
                        button(form="search", type="button", on:click=hide_preview) { "✖" }

                        (if content_type.starts_with("video") {
                            let content_type = content_type.clone();
                            let object_url = object_url.clone();

                            view! { cx,
                                video(id="preview_object", controls=true, autoplay=true) {
                                    source(src=object_url, type=content_type)

                                    p(style="text-align: center;") {
                                        "Предпросмотр файла не поддерживается"
                                    }
                                }
                            }
                        } else if content_type.starts_with("audio") {
                            let content_type = content_type.clone();
                            let object_url = object_url.clone();

                            view! { cx,
                                audio(id="preview_object", controls=true, autoplay=true) {
                                    source(src=object_url, type=content_type)

                                    p(style="text-align: center;") {
                                        "Предпросмотр файла не поддерживается"
                                    }
                                }
                            }
                        } else {
                            let content_type = content_type.clone();
                            let object_url = object_url.clone();

                            view! { cx,
                                object(id="preview_object", data=object_url, type=content_type) {
                                    p(style="text-align: center;") {
                                        "Предпросмотр файла не поддерживается"
                                    }
                                }
                            }
                        })
                    }
                }
            } else {
                view! { cx, }
            })
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
struct NumberFilterProps<'a, T> {
    legend: &'static str,
    id: &'static str,
    min: T,
    max: T,
    value_from: &'a Signal<Option<T>>,
    value_to: &'a Signal<Option<T>>,
    valid: &'a Signal<bool>,
}

#[component]
fn NumberFilter<'a, T, G>(cx: Scope<'a>, props: NumberFilterProps<'a, T>) -> View<G>
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

#[component(inline_props)]
fn SearchResults<'a, G: Html>(
    cx: Scope<'a>,
    search_results: &'a ReadSignal<Vec<FileES>>,
    display_preview: &'a Signal<bool>,
    preview_data: &'a Signal<PreviewData>,
    status_dialog_state: &'a Signal<StatusDialogState>,
) -> View<G> {
    view! { cx,
        Keyed(
            iterable=search_results,
            key=|item| item._id.clone().unwrap(),
            view=move |cx, item| {
                let file_name = item.path.file_name().unwrap().to_string_lossy().into_owned();
                let path = item.path.to_string_lossy().into_owned();
                let path_ = item.path.clone();
                let path__ = item.path.clone();
                let path___ = item.path.clone();
                let content_type = item.content_type.clone();

                let show_preview = move |_| {
                    preview_data.set(PreviewData {
                        path: item.path.clone(),
                        content_type: content_type.clone()
                    });
                    display_preview.set(true);
                };
                let open_path = move |path| {
                    spawn_local_scoped(cx, async move {
                        status_dialog_state.set(StatusDialogState::Loading);

                        if let Err(e) = open_path(&OpenPathArgs { path }).await {
                            status_dialog_state.set(StatusDialogState::Error(format!(
                                "❌ Ошибка открытия: {:#?}",
                                e
                            )));
                            return;
                        }
                        status_dialog_state.set(StatusDialogState::None);
                    })
                };
                let open_file = move |_| {
                    let path = path__.clone();
                    open_path(path)
                };
                let open_folder = move |_| {
                    let path = path___.parent().unwrap().to_path_buf();
                    open_path(path)
                };

                view! { cx,
                    article(class="search_result") {
                        (if item.content_type.starts_with("image")
                                || item.content_type.starts_with("video")
                                || item.content_type.starts_with("audio") {
                            let img_url = get_local_file_url(&path_, true);
                            view! { cx,
                                img(src=(img_url), onerror="this.style.display='none'") {}
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
                        div {
                            button(form="search", type="button", on:click=show_preview) { "Показать" }
                            button(form="search", type="button", on:click=open_file) { "Открыть" }
                            button(form="search", type="button", on:click=open_folder) { "Открыть папку" }
                        }

                        details {
                            summary { "Основные свойства файла" }

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

                        (if item.image_data.any_metadata() {
                            view! { cx,
                                details {
                                    summary { "Свойства изображения" }

                                    (if let Some(width) = item.image_data.width {
                                        view! { cx,
                                            p {
                                                "Ширина (пиксели): " (width)
                                            }
                                        }
                                    } else {
                                        view! { cx, }
                                    })
                                    (if let Some(height) = item.image_data.height {
                                        view! { cx,
                                            p {
                                                "Высота (пиксели): " (height)
                                            }
                                        }
                                    } else {
                                        view! { cx, }
                                    })
                                }
                            }
                        } else {
                            view! { cx, }
                        })

                        (if item.document_data.any_metadata() {
                            let document_data = item.document_data.clone();
                            view! { cx,
                                details {
                                    summary { "Свойства документа" }

                                    (if let Some(title) = document_data.title.clone() {
                                        view! { cx,
                                            p {
                                                "Заголовок: " (title)
                                            }
                                        }
                                    } else {
                                        view! { cx, }
                                    })
                                    (if let Some(creator) = document_data.creator.clone() {
                                        view! { cx,
                                            p {
                                                "Создатель: " (creator)
                                            }
                                        }
                                    } else {
                                        view! { cx, }
                                    })
                                    (if let Some(doc_created) = document_data.doc_created {
                                        view! { cx,
                                            p {
                                                "Создано: " (doc_created.with_timezone(&Local))
                                            }
                                        }
                                    } else {
                                        view! { cx, }
                                    })
                                    (if let Some(doc_modified) = document_data.doc_modified {
                                        view! { cx,
                                            p {
                                                "Изменено: " (doc_modified.with_timezone(&Local))
                                            }
                                        }
                                    } else {
                                        view! { cx, }
                                    })
                                    (if let Some(num_pages) = document_data.num_pages {
                                        view! { cx,
                                            p {
                                                "Страниц: " (num_pages)
                                            }
                                        }
                                    } else {
                                        view! { cx, }
                                    })
                                    (if let Some(num_words) = document_data.num_words {
                                        view! { cx,
                                            p {
                                                "Слов: " (num_words)
                                            }
                                        }
                                    } else {
                                        view! { cx, }
                                    })
                                    (if let Some(num_characters) = document_data.num_characters {
                                        view! { cx,
                                            p {
                                                "Символов: " (num_characters)
                                            }
                                        }
                                    } else {
                                        view! { cx, }
                                    })
                                }
                            }
                        } else {
                            view! { cx, }
                        })
                    }
                }
            }
        )
    }
}
