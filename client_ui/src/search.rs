use std::path::{Path, PathBuf};

use common_lib::{
    actions::PickFileResult,
    search::{
        DocumentSearchRequest, ImageQuery, ImageSearchRequest, PageType, SearchRequest,
        SearchResponse, TextQuery,
    },
};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use url::Url;
use wasm_bindgen::JsValue;
use web_sys::window;

use crate::{
    app::{fetch, widgets::StatusDialogState},
    search::{
        filters::{CheckboxFilter, DateTimeFilter, NumberFilter, RadioFilter},
        results::SearchResults,
    },
    settings::{MAX_FILE_SIZE_MAX, MAX_FILE_SIZE_MIN},
};

mod filters;
mod results;

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
    let path_enabled = create_signal(cx, true);
    let hash_enabled = create_signal(cx, true);
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

    let title_enabled = create_signal(cx, true);
    let creator_enabled = create_signal(cx, true);
    let doc_created_from = create_signal(cx, None);
    let doc_created_to = create_signal(cx, None);
    let doc_created_valid = create_signal(cx, true);
    let doc_modified_from = create_signal(cx, None);
    let doc_modified_to = create_signal(cx, None);
    let doc_modified_valid = create_signal(cx, true);
    let num_pages_from = create_signal(cx, None);
    let num_pages_to = create_signal(cx, None);
    let num_pages_valid = create_signal(cx, true);
    let num_words_from = create_signal(cx, None);
    let num_words_to = create_signal(cx, None);
    let num_words_valid = create_signal(cx, true);
    let num_characters_from = create_signal(cx, None);
    let num_characters_to = create_signal(cx, None);
    let num_characters_valid = create_signal(cx, true);

    let any_invalid = create_memo(cx, || {
        !*modified_valid.get()
            || !*size_valid.get()
            || !*width_valid.get()
            || !*height_valid.get()
            || !*doc_created_valid.get()
            || !*doc_modified_valid.get()
            || !*num_pages_valid.get()
            || !*num_words_valid.get()
            || !*num_characters_valid.get()
    });

    let display_preview = create_signal(cx, false);
    let preview_data = create_signal(cx, PreviewData::default());

    let search_results = create_signal(cx, Vec::new());
    let pages = create_signal(cx, Vec::new());

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

    let search = move |page: u32| {
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
                page,
                query: search_query,
                path_enabled: *path_enabled.get(),
                hash_enabled: *hash_enabled.get(),
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
                    title_enabled: *title_enabled.get(),
                    creator_enabled: *creator_enabled.get(),
                    doc_created_from: *doc_created_from.get(),
                    doc_created_to: *doc_created_to.get(),
                    doc_modified_from: *doc_modified_from.get(),
                    doc_modified_to: *doc_modified_to.get(),
                    num_pages_from: *num_pages_from.get(),
                    num_pages_to: *num_pages_to.get(),
                    num_words_from: *num_words_from.get(),
                    num_words_to: *num_words_to.get(),
                    num_characters_from: *num_characters_from.get(),
                    num_characters_to: *num_characters_to.get(),
                },
            };

            match search(&search_request).await {
                Ok(x) => {
                    search_results.set(x.results);
                    pages.set(x.pages);
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

                        fieldset {
                            legend { "Текстовый поиск" }
                            CheckboxFilter(text="Заголовок", id="title", value_enabled=title_enabled)
                            CheckboxFilter(text="Создатель", id="creator", value_enabled=creator_enabled)
                        }

                        DateTimeFilter(legend="Дата и время создания", id="doc_created",
                            value_from=doc_created_from, value_to=doc_created_to, valid=doc_created_valid)

                        DateTimeFilter(legend="Дата и время изменения", id="doc_modified",
                            value_from=doc_modified_from, value_to=doc_modified_to, valid=doc_modified_valid)

                        NumberFilter(legend="Количество страниц", id="num_pages",
                            min=1, max=u32::MAX, value_from=num_pages_from, value_to=num_pages_to,
                            valid=num_pages_valid)

                        NumberFilter(legend="Количество слов", id="num_words",
                            min=1, max=u32::MAX, value_from=num_words_from, value_to=num_words_to,
                            valid=num_words_valid)

                        NumberFilter(legend="Количество символов", id="num_characters",
                            min=1, max=u32::MAX, value_from=num_characters_from, value_to=num_characters_to,
                            valid=num_characters_valid)
                    }
                }
            }

            main {
                SearchResults(search_results=search_results, display_preview=display_preview,
                    preview_data=preview_data, status_dialog_state=status_dialog_state)

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
