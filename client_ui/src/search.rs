use std::path::{Path, PathBuf};

use common_lib::{
    actions::PickFileResult,
    search::{ImageQuery, PageType, SearchRequest, SearchResponse, TextQuery},
    settings::Settings,
};
use gloo_net::http::Request;
use sycamore::{futures::spawn_local_scoped, prelude::*};
use url::Url;
use wasm_bindgen::JsValue;
use web_sys::window;

use crate::{
    app::{fetch, fetch_empty, widgets::StatusDialogState},
    search::{
        filters::{
            content_type::{
                content_type_filter_items, get_content_type_request_items,
                load_from_content_type_request_items, ContentTypeFilter,
            },
            CheckboxFilter, DateTimeFilter, NumberFilter, RadioFilter, RangeWidget,
        },
        results::SearchResults,
    },
    settings::{MAX_FILE_SIZE_MAX, MAX_FILE_SIZE_MIN},
};

use self::{
    filter_groups::{
        DocumentFilters, DocumentFiltersData, ImageFilters, ImageFiltersData, MultimediaFilters,
        MultimediaFiltersData,
    },
    filters::PathFilter,
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

async fn open_request() -> Result<Option<SearchRequest>, JsValue> {
    fetch("/open_request", "POST", None::<&()>).await
}

async fn save_request(search_request: &SearchRequest) -> Result<(), JsValue> {
    fetch_empty("/save_request", "POST", Some(search_request)).await
}

async fn search(search_request: &SearchRequest) -> Result<SearchResponse, JsValue> {
    fetch("/search", "POST", Some(search_request)).await
}

#[component(inline_props)]
pub fn Search<'a, G: Html>(
    cx: Scope<'a>,
    settings: &'a Signal<Settings>,
    status_dialog_state: &'a Signal<StatusDialogState>,
) -> View<G> {
    let query = create_signal(cx, String::new());
    let query_image_path = create_signal(cx, PathBuf::new());

    let query_type = create_signal(cx, QueryType::Text);
    let content_enabled = create_signal(cx, true);
    let text_search_enabled = create_signal(cx, settings.get().nn_server.text_search_enabled);
    let image_search_enabled = create_signal(cx, settings.get().nn_server.image_search_enabled);
    let reranking_enabled = create_signal(cx, settings.get().nn_server.reranking_enabled);
    let text_search_pages = create_signal(cx, 1);
    let image_search_pages = create_signal(cx, 1);
    let query_coeff = create_signal(cx, 1.0);
    let text_search_coeff = create_signal(cx, 7.5);
    let image_search_coeff = create_signal(cx, 7.5);
    let reranking_coeff = create_signal(cx, 1.1);

    let display_filters = create_signal(cx, true);
    let path_prefix = create_signal(cx, None);
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

    let no_searches = create_signal(cx, true);
    let search_results = create_signal(cx, Vec::new());
    let pages = create_signal(cx, Vec::new());
    let suggestion = create_signal(cx, None);

    // Update search configuration on settings change
    create_effect(cx, || {
        text_search_enabled.set(settings.get().nn_server.text_search_enabled);
        image_search_enabled.set(settings.get().nn_server.image_search_enabled);
        reranking_enabled.set(settings.get().nn_server.reranking_enabled);
    });

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
                        "❌ Ошибка открытия диалога: {e:#?}",
                    )));
                }
            }
        });
    };

    let get_search_request = |page: u32| {
        let search_query = match *query_type.get() {
            QueryType::Text => common_lib::search::QueryType::Text(TextQuery {
                query: (*query.get()).clone(),
                content_enabled: *content_enabled.get(),
                text_search_enabled: *text_search_enabled.get(),
                image_search_enabled: *image_search_enabled.get(),
                reranking_enabled: *reranking_enabled.get(),
                text_search_pages: *text_search_pages.get(),
                image_search_pages: *image_search_pages.get(),
                query_coeff: *query_coeff.get(),
                text_search_coeff: *text_search_coeff.get(),
                image_search_coeff: *image_search_coeff.get(),
                reranking_coeff: *reranking_coeff.get(),
            }),
            QueryType::Image => common_lib::search::QueryType::Image(ImageQuery {
                image_path: (*query_image_path.get()).clone(),
                image_search_pages: *image_search_pages.get(),
            }),
        };
        SearchRequest {
            page,
            query: search_query,
            path_prefix: path_prefix.get().as_ref().clone(),
            content_type: (!*content_type_disabled.get())
                .then(|| get_content_type_request_items(content_type_items)),
            path_enabled: *path_enabled.get(),
            hash_enabled: *hash_enabled.get(),
            modified_from: *modified_from.get(),
            modified_to: *modified_to.get(),
            size_from: size_from.get().map(|x| (x * 1024.0 * 1024.0) as u64),
            size_to: size_to.get().map(|x| (x * 1024.0 * 1024.0) as u64),
            image_data: image_filters_data.get().to_request(),
            multimedia_data: multimedia_filters_data.get().to_request(),
            document_data: document_filters_data.get().to_request(),
        }
    };

    let load_from_search_request = |search_request: SearchRequest| {
        match search_request.query {
            common_lib::search::QueryType::Text(text_query) => {
                query.set(text_query.query);
                content_enabled.set(text_query.content_enabled);
                text_search_enabled.set(text_query.text_search_enabled);
                image_search_enabled.set(text_query.image_search_enabled);
                reranking_enabled.set(text_query.reranking_enabled);
                text_search_pages.set(text_query.text_search_pages);
                image_search_pages.set(text_query.image_search_pages);
                query_coeff.set(text_query.query_coeff);
                text_search_coeff.set(text_query.text_search_coeff);
                image_search_coeff.set(text_query.image_search_coeff);
                reranking_coeff.set(text_query.reranking_coeff);
            }
            common_lib::search::QueryType::Image(image_query) => {
                query_image_path.set(image_query.image_path);
                image_search_pages.set(image_query.image_search_pages);
            }
        };
        path_prefix.set(search_request.path_prefix);
        match search_request.content_type {
            Some(x) => {
                content_type_disabled.set(false);
                load_from_content_type_request_items(&x, content_type_items);
            }
            None => content_type_disabled.set(true),
        }
        path_enabled.set(search_request.path_enabled);
        hash_enabled.set(search_request.hash_enabled);
        modified_from.set(search_request.modified_from);
        modified_to.set(search_request.modified_to);
        size_from.set(
            search_request
                .size_from
                .map(|x| (x as f64) / 1024.0 / 1024.0),
        );
        size_to.set(search_request.size_to.map(|x| (x as f64) / 1024.0 / 1024.0));
        image_filters_data
            .modify()
            .update_from_request(search_request.image_data);
        multimedia_filters_data
            .modify()
            .update_from_request(search_request.multimedia_data);
        document_filters_data
            .modify()
            .update_from_request(search_request.document_data);
    };

    let open_search_request = move |_| {
        spawn_local_scoped(cx, async move {
            status_dialog_state.set(StatusDialogState::Loading);

            match open_request().await {
                Ok(res) => {
                    if let Some(search_request) = res {
                        load_from_search_request(search_request);
                    }
                    status_dialog_state.set(StatusDialogState::None);
                }
                Err(e) => {
                    status_dialog_state.set(StatusDialogState::Error(format!(
                        "❌ Ошибка открытия запроса: {e:#?}",
                    )));
                }
            }
        });
    };
    let save_search_request = move |_| {
        let search_request = get_search_request(0);
        spawn_local_scoped(cx, async move {
            status_dialog_state.set(StatusDialogState::Loading);

            match save_request(&search_request).await {
                Ok(_) => {
                    status_dialog_state.set(StatusDialogState::None);
                }
                Err(e) => {
                    status_dialog_state.set(StatusDialogState::Error(format!(
                        "❌ Ошибка сохранения запроса: {e:#?}",
                    )));
                }
            }
        });
    };

    let search = move |page: u32| {
        spawn_local_scoped(cx, async move {
            no_searches.set(false);
            status_dialog_state.set(StatusDialogState::Loading);

            let search_request = get_search_request(page);

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
                        "❌ Ошибка поиска: {e:#?}",
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
                        legend { "Сохранённые запросы" }
                        div(id="saved_requests") {
                            button(form="search", type="button", on:click=open_search_request) { "Открыть" }
                            button(form="search", type="button", on:click=save_search_request) { "Сохранить" }
                        }
                    }
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
                                    CheckboxFilter(text="Переранжирование", id="reranking",
                                        value_enabled=reranking_enabled)
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
                                    RangeWidget(legend="Переранжирование", id="reranking_coeff",
                                        min=0.1, max=5.0, step=0.1, value=reranking_coeff)
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

                    PathFilter(legend="Искать в папке", id="path_prefix", value=path_prefix,
                        status_dialog_state=status_dialog_state)

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

                (if *no_searches.get() {
                    view! { cx,
                        div(style="text-align: center;") {
                            p { "Перед началом работы выберите индексируемые папки на вкладке \"Настройки\" и сохраните их." }
                            p { "Затем проиндексируйте их на вкладке \"Индексация\"." }
                            p { "Для поиска выберите тип запроса слева, введите текст запроса или выберите изображение выше." }
                            p { "При необходимости выберите тип поиска, тип файлов, папку поиска, дополнительные фильтры слева." }
                        }
                    }
                } else {
                    view! { cx,
                        (if search_results.get().is_empty() {
                            view! { cx,
                                h3(style="text-align: center;") { "Ничего не найдено" }
                            }
                        } else {
                            view! { cx,
                                SearchResults(search_results=search_results, preview_data=preview_data,
                                    status_dialog_state=status_dialog_state)
                                Pagination(pages=pages, search=search)
                            }
                        })
                    }
                })
            }

            Preview(preview_data=preview_data, status_dialog_state=status_dialog_state)
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
    id: String,
}

#[component(inline_props)]
fn Preview<'a, G: Html>(
    cx: Scope<'a>,
    preview_data: &'a Signal<PreviewData>,
    status_dialog_state: &'a Signal<StatusDialogState>,
) -> View<G> {
    let hide_preview = move |_| {
        preview_data.modify().display = false;
    };

    view! { cx,
        (if preview_data.get().display {
            let content_type = preview_data.get().content_type.clone();
            let object_url = get_local_file_url(&preview_data.get().path, Some(&content_type), false);
            let id = preview_data.get().id.clone();

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
                    } else if content_type != "text/html" && content_type != "application/pdf" {
                        let id = id.clone();
                        spawn_local_scoped(cx, async move {
                            let content = match Request::get("/document_content")
                                .query([("id", id)])
                                .send()
                                .await
                            {
                                Ok(response) => response.text().await,
                                Err(e) => Err(e),
                            };
                            match content {
                                Ok(content) => {
                                    let element = web_sys::window()
                                        .expect("`window` not found")
                                        .document()
                                        .expect("`document` not found")
                                        .get_element_by_id("preview_object")
                                        .expect("`preview_object` not found");
                                    element.set_text_content(Some(&content));
                                    status_dialog_state.set(StatusDialogState::None);
                                }
                                Err(e) => {
                                    status_dialog_state.set(StatusDialogState::Error(format!(
                                        "❌ Ошибка получения файла: {e:#?}",
                                    )));
                                }
                            }
                        });

                        view! { cx,
                            pre(id="preview_object", style="overflow: scroll; white-space: pre-wrap;")
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
