use common_lib::indexer::{IndexStats, IndexingStatus, IndexingWSMessage};
use futures::StreamExt;
use gloo_net::websocket::{futures::WebSocket, Message};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use url::Url;
use wasm_bindgen::JsValue;

use crate::{
    app::{fetch_empty, widgets::StatusDialogState},
    formatting::{duration_str_from_seconds, file_size_str},
};

async fn index() -> Result<(), JsValue> {
    fetch_empty("/index", "PATCH", None::<&()>).await
}

async fn delete_index() -> Result<(), JsValue> {
    fetch_empty("/index", "DELETE", None::<&()>).await
}

#[component(inline_props)]
pub fn Status<'a, G: Html>(
    cx: Scope<'a>,
    status_dialog_state: &'a Signal<StatusDialogState>,
) -> View<G> {
    let indexing_status = create_signal(cx, IndexingStatus::NotStarted);
    let index_stats = create_signal(cx, IndexStats::default());

    let is_indexing = create_memo(cx, || !indexing_status.get().can_start());

    spawn_local_scoped(cx, async move {
        status_dialog_state.set(StatusDialogState::Loading);

        let mut ws_url =
            Url::parse(&web_sys::window().unwrap().location().origin().unwrap()).unwrap();
        ws_url.set_scheme("ws").unwrap();
        ws_url.set_path("/index");
        let ws = WebSocket::open(ws_url.as_str()).unwrap();
        let (_, mut ws_read) = ws.split();
        spawn_local_scoped(cx, async move {
            if let Err(e) = async {
                while let Some(msg) = ws_read.next().await {
                    match msg.map_err(|e| e.to_string())? {
                        Message::Text(msg) => {
                            let msg: IndexingWSMessage = serde_json::from_str(&msg).unwrap();
                            match msg {
                                IndexingWSMessage::IndexingStatus(x) => indexing_status.set(x),
                                IndexingWSMessage::IndexingEvent(x) => {
                                    indexing_status.modify().process_event(x)
                                }
                                IndexingWSMessage::IndexStats(x) => index_stats.set(x),
                                IndexingWSMessage::Error(e) => return Err(e),
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                Ok::<_, String>(())
            }
            .await
            {
                status_dialog_state.set(StatusDialogState::Error(format!(
                    "❌ Ошибка загрузки статуса индексирования: {}",
                    e
                )));
            }
        });
    });

    let index = move |_| {
        spawn_local_scoped(cx, async move {
            status_dialog_state.set(StatusDialogState::Loading);

            match index().await {
                Ok(_) => {
                    status_dialog_state.set(StatusDialogState::None);
                }
                Err(e) => {
                    status_dialog_state.set(StatusDialogState::Error(format!(
                        "❌ Ошибка индексирования: {:#?}",
                        e,
                    )));
                }
            }
        })
    };

    let delete_index = move |_| {
        spawn_local_scoped(cx, async move {
            status_dialog_state.set(StatusDialogState::Loading);

            match delete_index().await {
                Ok(_) => {
                    status_dialog_state.set(StatusDialogState::None);
                }
                Err(e) => {
                    status_dialog_state.set(StatusDialogState::Error(format!(
                        "❌ Ошибка очищения индекса: {:#?}",
                        e,
                    )));
                }
            }
        })
    };

    view! { cx,
        div(class="main_container") {
            main {
                form(id="status", on:submit=index, action="javascript:void(0);") {
                    fieldset {
                        legend { "Индексация" }
                        p {
                            "Статус: " (indexing_status.get())
                        }
                        (if let IndexingStatus::Finished(_) = *indexing_status.get() {
                            view! { cx,
                                p {
                                    "Результаты последней индексации:"
                                }
                            }
                        } else {
                            view! { cx, }
                        })
                        (match (*indexing_status.get()).clone() {
                            IndexingStatus::Indexing(mut data) | IndexingStatus::Finished(mut data) => {
                                const MAX_ERROR_CNT: usize = 20;

                                let errors_cnt = data.errors.len();
                                data.errors.truncate(MAX_ERROR_CNT);
                                let errors = create_signal(cx, data.errors);
                                view! { cx,
                                    p {
                                        "Добавление " (data.to_add) ", удаление " (data.to_remove)
                                        ", обновление " (data.to_update) " файлов в индексе"
                                    }
                                    p {
                                        "Обработано " (data.processed) " файлов, загружено "
                                        (data.sent) " изменений"
                                    }
                                    (if let Some(duration) = data.duration {
                                        let duration_str = duration_str_from_seconds(duration.as_secs_f32());
                                        view! { cx, p { "Прошло " (duration_str) } }
                                    } else {
                                        view! { cx, }
                                    })
                                    Keyed(
                                        iterable=errors,
                                        key=|e| e.to_owned(),
                                        view=move |cx, e| {
                                            view! { cx,
                                                p {
                                                    "❌ Ошибка индексации: " (e)
                                                }
                                            }
                                        }
                                    )
                                    (if errors_cnt > MAX_ERROR_CNT {
                                        view! { cx,
                                            p {
                                                "(ещё " (errors_cnt - MAX_ERROR_CNT) " ошибок)"
                                            }
                                        }
                                    } else {
                                        view! { cx, }
                                    })
                                }
                            }
                            _ => {
                                view! { cx, }
                            }
                        })
                    }
                    fieldset {
                        legend { "Статистика" }
                        p {
                            "Количество файлов в индексе: " (index_stats.get().doc_cnt)
                        }
                        p {
                            "Размер индекса: " (file_size_str(index_stats.get().index_size))
                        }
                    }

                    div(class="settings_buttons") {
                        button(type="button", on:click=delete_index, disabled=*is_indexing.get()) { "Очистить индекс" }
                        button(type="submit", disabled=*is_indexing.get()) { "Индексировать" }
                    }
                }
            }
        }
    }
}
