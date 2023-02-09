use std::path::{Path, PathBuf};

use common_lib::{
    actions::PickFileResult,
    search::{ImageQuery, PageType, SearchRequest, SearchResponse, TextQuery},
};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use url::Url;
use wasm_bindgen::JsValue;
use web_sys::window;

use crate::{
    app::{fetch, widgets::StatusDialogState},
    search::{
        filters::{
            content_type::{
                content_type_filter_items, content_type_request_items, ContentTypeFilter,
            },
            CheckboxFilter, DateTimeFilter, NumberFilter, RadioFilter, RangeWidget,
        },
        results::SearchResults,
    },
    settings::{MAX_FILE_SIZE_MAX, MAX_FILE_SIZE_MIN},
};

use self::filter_groups::{
    DocumentFilters, DocumentFiltersData, ImageFilters, ImageFiltersData, MultimediaFilters,
    MultimediaFiltersData,
};

mod filter_groups;
mod filters;
mod results;

#[derive(Debug, Clone, Copy)]
enum QueryType {
    Text,
    Image,
}

fn get_local_file_url<P: AsRef<Path>>(path: P, content_type: Option<&str>, thumbnail: bool) -> Url {
    let base = Url::parse(&web_sys::window().unwrap().location().origin().unwrap()).unwrap();
    let mut file_url = base.join("/file").unwrap();
    file_url
        .query_pairs_mut()
        .append_pair("path", &path.as_ref().to_string_lossy())
        .append_pair("thumbnail", &thumbnail.to_string());
    if let Some(x) = content_type {
        file_url.query_pairs_mut().append_pair("content_type", x);
    }
    file_url
}

async fn pick_file() -> Result<PickFileResult, JsValue> {
    fetch("/pick_file", "POST", None::<&()>).await
}

async fn search(search_request: &SearchRequest) -> Result<SearchResponse, JsValue> {
    fetch("/search", "POST", Some(search_request)).await
}

#[component(inline_props)]
pub fn Search<'a, G: Html>(
    cx: Scope<'a>,
    status_dialog_state: &'a Signal<StatusDialogState>,
) -> View<G> {
    let query = create_signal(cx, String::new());
    let query_image_path = create_signal(cx, PathBuf::new());

    let query_type = create_signal(cx, QueryType::Text);
    let content_enabled = create_signal(cx, true);
    let text_search_enabled = create_signal(cx, true);
    let image_search_enabled = create_signal(cx, true);
    let text_search_pages = create_signal(cx, 1);
    let image_search_pages = create_signal(cx, 1);
    let query_coeff = create_signal(cx, 1.0);
    let text_search_coeff = create_signal(cx, 2.0);
    let image_search_coeff = create_signal(cx, 2.0);

    let display_filters = create_signal(cx, true);
    let content_type_disabled = create_signal(cx, true);
    let content_type_items = content_type_filter_items(cx);
    let path_enabled = create_signal(cx, true);
    let hash_enabled = create_signal(cx, true);
    let modified_from = create_signal(cx, None);
    let modified_to = create_signal(cx, None);
    let modified_valid = create_signal(cx, true);
    let size_from = create_signal(cx, None);
    let size_to = create_signal(cx, None);
    let size_valid = create_signal(cx, true);

    let image_filters_data = create_signal(cx, ImageFiltersData::new(cx));
    let multimedia_filters_data = create_signal(cx, MultimediaFiltersData::new(cx));
    let document_filters_data = create_signal(cx, DocumentFiltersData::new(cx));

    let any_invalid = create_memo(cx, || {
        !*modified_valid.get()
            || !*size_valid.get()
            || *image_filters_data.get().any_invalid.get()
            || *multimedia_filters_data.get().any_invalid.get()
            || *document_filters_data.get().any_invalid.get()
    });

    let preview_data = create_signal(cx, PreviewData::default());

    let search_results = create_signal(cx, Vec::new());
    let pages = create_signal(cx, Vec::new());
    let suggestion = create_signal(cx, None);

    let toggle_filters = move |_| {
        display_filters.set(!*display_filters.get());
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

    let search = move |page: u32| {
        spawn_local_scoped(cx, async move {
            status_dialog_state.set(StatusDialogState::Loading);

            let search_query = match *query_type.get() {
                QueryType::Text => common_lib::search::QueryType::Text(TextQuery {
                    query: (*query.get()).clone(),
                    content_enabled: *content_enabled.get(),
                    text_search_enabled: *text_search_enabled.get(),
                    image_search_enabled: *image_search_enabled.get(),
                    text_search_pages: *text_search_pages.get(),
                    image_search_pages: *image_search_pages.get(),
                    query_coeff: *query_coeff.get(),
                    text_search_coeff: *text_search_coeff.get(),
                    image_search_coeff: *image_search_coeff.get(),
                }),
                QueryType::Image => common_lib::search::QueryType::Image(ImageQuery {
                    image_path: (*query_image_path.get()).clone(),
                    image_search_pages: *image_search_pages.get(),
                }),
            };
            let search_request = SearchRequest {
                page,
                query: search_query,
                content_type: (!*content_type_disabled.get())
                    .then(|| content_type_request_items(content_type_items)),
                path_enabled: *path_enabled.get(),
                hash_enabled: *hash_enabled.get(),
                modified_from: *modified_from.get(),
                modified_to: *modified_to.get(),
                size_from: size_from.get().map(|x| (x * 1024.0 * 1024.0) as u64),
                size_to: size_to.get().map(|x| (x * 1024.0 * 1024.0) as u64),
                image_data: image_filters_data.get().to_request(),
                multimedia_data: multimedia_filters_data.get().to_request(),
                document_data: document_filters_data.get().to_request(),
            };

            match search(&search_request).await {
                Ok(x) => {
                    search_results.set(x.results);
                    pages.set(x.pages);
                    suggestion.set(x.suggestion);
                    status_dialog_state.set(StatusDialogState::None);
                    window().unwrap().scroll_to_with_x_and_y(0.0, 0.0);
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
    let search_without_page = move |_| search(0);

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
                            let img_url = get_local_file_url(&*query_image_path.get(), None, false);
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
                form(id="search", on:submit=search_without_page, action="javascript:void(0);") {
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
                                    CheckboxFilter(text="Поиск по содержимому", id="content", value_enabled=content_enabled)
                                    CheckboxFilter(text="Семантический поиск по тексту", id="text_search",
                                        value_enabled=text_search_enabled)
                                    CheckboxFilter(text="Семантический поиск по изображениям", id="image_search",
                                        value_enabled=image_search_enabled)
                                }

                                details {
                                    summary { "Количество страниц семантического поиска" }

                                    RangeWidget(legend="По тексту", id="text_search_pages",
                                        min=1, max=20, step=1, value=text_search_pages)
                                    RangeWidget(legend="По изображениям", id="image_search_pages",
                                        min=1, max=20, step=1, value=image_search_pages)
                                }

                                details {
                                    summary { "Коэффициенты поиска" }

                                    RangeWidget(legend="По содержимому", id="query_coeff",
                                        min=1.0, max=10.0, step=0.1, value=query_coeff)
                                    RangeWidget(legend="Семантический по тексту", id="text_search_coeff",
                                        min=1.0, max=10.0, step=0.1, value=text_search_coeff)
                                    RangeWidget(legend="Семантический по изображениям", id="image_search_coeff",
                                        min=1.0, max=10.0, step=0.1, value=image_search_coeff)
                                }
                            }
                        }
                        QueryType::Image => {
                            view! { cx,
                                details {
                                    summary { "Количество страниц семантического поиска" }

                                    RangeWidget(legend="По изображениям", id="image_search_pages",
                                        min=1, max=20, step=1, value=image_search_pages)
                                }
                            }
                        }
                    })

                    ContentTypeFilter(items=content_type_items, disabled=content_type_disabled)

                    details {
                        summary { "Основные свойства файла" }

                        fieldset {
                            legend { "Текстовый поиск" }
                            CheckboxFilter(text="Путь файла", id="path", value_enabled=path_enabled)
                            CheckboxFilter(text="Хеш", id="hash", value_enabled=hash_enabled)
                        }

                        DateTimeFilter(legend="Дата и время изменения", id="modified",
                            value_from=modified_from, value_to=modified_to, valid=modified_valid)

                        NumberFilter(legend="Размер файла (МиБ)", id="size",
                            min=MAX_FILE_SIZE_MIN, max=MAX_FILE_SIZE_MAX,
                            value_from=size_from, value_to=size_to, valid=size_valid)
                    }

                    ImageFilters(data=image_filters_data)

                    MultimediaFilters(data=multimedia_filters_data)

                    DocumentFilters(data=document_filters_data)
                }
            }

            main {
                (if let Some((highlight, text)) = (*suggestion.get()).clone() {
                    let change_query = move |e| {
                        query.set(text.clone());
                        search_without_page(e);
                    };

                    view! { cx,
                        h3 {
                            "Возможный запрос: "
                            a(on:click=change_query, href="javascript:void(0);",
                                dangerously_set_inner_html=&highlight)
                        }
                    }
                } else {
                    view! { cx, }
                })

                SearchResults(search_results=search_results, preview_data=preview_data,
                    status_dialog_state=status_dialog_state)

                Pagination(pages=pages, search=search)

            }

            Preview(preview_data=preview_data)
        }
    }
}

#[component(inline_props)]
fn Pagination<'a, F, G>(cx: Scope<'a>, pages: &'a ReadSignal<Vec<PageType>>, search: F) -> View<G>
where
    F: Fn(u32) + Copy + 'a,
    G: Html,
{
    view! { cx,
        div(id="pagination") {
            Keyed(
                iterable=pages,
                key=|x| *x,
                view=move |cx, x| {
                    let text = match x {
                        PageType::First => "<< Первая".to_owned(),
                        PageType::Previous(_) => "< Предыдущая".to_owned(),
                        PageType::Next(_) => "Следующая >".to_owned(),
                        PageType::Last(_) => "Последняя >>".to_owned(),
                        PageType::Current(p) | PageType::Other(p) => (p + 1).to_string(),
                    };

                    let switch_page = move |_| {
                        let page = match x {
                            PageType::First => 0,
                            PageType::Previous(p) | PageType::Next(p) | PageType::Last(p)
                                | PageType::Current(p) | PageType::Other(p) => p,
                        };
                        search(page);
                    };

                    match x {
                        PageType::Current(_) => {
                            view! { cx, (text) " " }
                        }
                        _ => {
                            view! { cx,
                                a(on:click=switch_page, href="javascript:void(0);") { (text) }
                                " "
                            }
                        }
                    }
                }
            )
        }
    }
}

#[derive(Debug, Clone, Default)]
struct PreviewData {
    display: bool,
    path: PathBuf,
    content_type: String,
}

#[component(inline_props)]
fn Preview<'a, G: Html>(cx: Scope<'a>, preview_data: &'a Signal<PreviewData>) -> View<G> {
    let hide_preview = move |_| {
        preview_data.modify().display = false;
    };

    view! { cx,
        (if preview_data.get().display {
            let content_type = preview_data.get().content_type.clone();
            let object_url = get_local_file_url(&preview_data.get().path, Some(&content_type), false);
            view! { cx,
                aside(id="preview") {
                    button(form="search", type="button", on:click=hide_preview) { "✖" }

                    (if content_type.starts_with("image") {
                        let object_url = object_url.clone();

                        view! { cx,
                            img(id="preview_object", src=object_url)
                        }
                    } else if content_type.starts_with("video") {
                        let object_url = object_url.clone();

                        view! { cx,
                            video(id="preview_object", controls=true, autoplay=true) {
                                source(src=object_url)

                                p(style="text-align: center;") {
                                    "Предпросмотр файла не поддерживается"
                                }
                            }
                        }
                    } else if content_type.starts_with("audio") {
                        let object_url = object_url.clone();

                        view! { cx,
                            audio(id="preview_object", controls=true, autoplay=true) {
                                source(src=object_url)

                                p(style="text-align: center;") {
                                    "Предпросмотр файла не поддерживается"
                                }
                            }
                        }
                    } else {
                        let object_url = object_url.clone();

                        view! { cx,
                            object(id="preview_object", data=object_url) {
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
