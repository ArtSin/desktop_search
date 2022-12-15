use chrono::Local;
use common_lib::{
    elasticsearch::FileES,
    search::{SearchRequest, SearchResponse},
};
use serde::Serialize;
use serde_wasm_bindgen::{from_value, to_value};
use sycamore::{futures::spawn_local_scoped, prelude::*};

use crate::app::{invoke, StatusMessage};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchRequestArgs<'a> {
    search_request: &'a SearchRequest,
}

#[component]
pub fn Search<G: Html>(cx: Scope) -> View<G> {
    let status_str = create_signal(cx, String::new());
    let query = create_signal(cx, String::new());
    let search_results = create_signal(cx, Vec::new());

    let search = move |_| {
        spawn_local_scoped(cx, async move {
            status_str.set("⏳ Загрузка...".to_owned());

            let search_request = SearchRequest {
                query: (*query.get()).clone(),
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
                    status_str.set("❌ Ошибка поиска: ".to_owned() + &e);
                }
            }
        })
    };

    view! { cx,
        header {
            input(form="search", type="search", id="query", name="query",
                placeholder="Искать...", bind:value=query)
            button(form="search", type="submit") { "Искать" }
        }
        div(class="main_container") {
            aside {
                form(id="search", on:submit=search, action="javascript:void(0);") {

                }
            }
            main {
                StatusMessage(status_str=status_str)
                SearchResults(search_results=search_results)
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

                view! { cx,
                    article(class="search_result") {
                        h3 {
                            (file_name)
                        }
                        p {
                            "Полный путь: " (path)
                        }
                        p {
                            "Изменено: " (item.modified.with_timezone(&Local))
                        }
                        p {
                            "Размер (в байтах): " (item.size)
                        }
                        p {
                            "Хеш SHA-256: " (item.hash)
                        }
                    }
                }
            },
            key=|item| item._id.clone().unwrap(),
        )
    }
}
