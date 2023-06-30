use common_lib::indexer::{IndexStats, IndexingStatus, IndexingWSMessage, MAX_ERROR_CNT};
use fluent_bundle::FluentArgs;
use futures::StreamExt;
use gloo_net::websocket::{futures::WebSocket, Message};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use url::Url;
use wasm_bindgen::JsValue;

use crate::{
    app::{fetch_empty, get_translation, widgets::StatusDialogState},
    formatting::{duration_str_from_seconds, file_size_str},
};

fn indexing_status_str(status: &IndexingStatus) -> String {
    match status {
        IndexingStatus::NotStarted | IndexingStatus::Finished(_) => {
            get_translation("indexing_status_no_indexing", None).to_string()
        }
        IndexingStatus::DiffFailed(e) => {
            let error_args = FluentArgs::from_iter([("error", e.to_owned())]);
            get_translation("indexing_status_diff_failed", Some(&error_args)).to_string()
        }
        IndexingStatus::CalculatingDiff => {
            get_translation("indexing_status_calculating_diff", None).to_string()
        }
        IndexingStatus::Indexing(_) => {
            get_translation("indexing_status_indexing", None).to_string()
        }
    }
}

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
                let error_args = FluentArgs::from_iter([("error", e)]);
                let error_str =
                    get_translation("indexing_status_loading_error", Some(&error_args)).to_string();
                status_dialog_state.set(StatusDialogState::Error(error_str));
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
                    let error_args = FluentArgs::from_iter([("error", format!("{e:#?}"))]);
                    let error_str =
                        get_translation("indexing_error", Some(&error_args)).to_string();
                    status_dialog_state.set(StatusDialogState::Error(error_str));
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
                    let error_args = FluentArgs::from_iter([("error", format!("{e:#?}"))]);
                    let error_str =
                        get_translation("index_clearing_error", Some(&error_args)).to_string();
                    status_dialog_state.set(StatusDialogState::Error(error_str));
                }
            }
        })
    };

    view! { cx,
        div(class="main_container") {
            main {
                form(id="status", on:submit=index, action="javascript:void(0);") {
                    fieldset {
                        legend { (get_translation("indexing", None)) }
                        p {
                            (get_translation("indexing_status", Some(&FluentArgs::from_iter([("status", indexing_status_str(&indexing_status.get()))]))).to_string())
                        }
                        (if let IndexingStatus::Finished(_) = *indexing_status.get() {
                            view! { cx,
                                p { (get_translation("indexing_results", None)) }
                            }
                        } else {
                            view! { cx, }
                        })
                        (match (*indexing_status.get()).clone() {
                            IndexingStatus::Indexing(data) | IndexingStatus::Finished(data) => {
                                let errors = create_signal(cx, data.errors);

                                let add_remove_update_args = FluentArgs::from_iter([("to_add", data.to_add), ("to_remove", data.to_remove), ("to_update", data.to_update)]);
                                let add_remove_update_str = get_translation("indexing_add_remove_update", Some(&add_remove_update_args)).to_string();

                                let processed_sent_args = FluentArgs::from_iter([("processed", data.processed), ("sent", data.sent)]);
                                let processed_sent_str = get_translation("indexing_processed_sent", Some(&processed_sent_args)).to_string();

                                view! { cx,
                                    p { (add_remove_update_str) }
                                    p { (processed_sent_str) }
                                    (if let Some(duration) = data.duration {
                                        let duration_str = duration_str_from_seconds(duration.as_secs_f32());
                                        let elapsed_args = FluentArgs::from_iter([("duration", duration_str)]);
                                        let elapsed_str = get_translation("indexing_elapsed", Some(&elapsed_args)).to_string();

                                        view! { cx, p { (elapsed_str) } }
                                    } else {
                                        view! { cx, }
                                    })
                                    Keyed(
                                        iterable=errors,
                                        key=|e| e.to_owned(),
                                        view=move |cx, e| {
                                            let error_args = FluentArgs::from_iter([("error", e)]);
                                            let error_str = get_translation("indexing_error", Some(&error_args)).to_string();

                                            view! { cx, p { (error_str) } }
                                        }
                                    )
                                    (if data.errors_cnt > MAX_ERROR_CNT {
                                        let more_errors_args = FluentArgs::from_iter([("count", data.errors_cnt - MAX_ERROR_CNT)]);
                                        let more_errors_str = get_translation("indexing_more_errors", Some(&more_errors_args)).to_string();

                                        view! { cx, p { (more_errors_str) } }
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
                        legend { (get_translation("indexing_statistics", None)) }
                        p {
                            (get_translation("indexing_doc_cnt", Some(&FluentArgs::from_iter([("count", index_stats.get().doc_cnt)]))).to_string())
                        }
                        p {
                            (get_translation("indexing_index_size", Some(&FluentArgs::from_iter([("size", file_size_str(index_stats.get().index_size))]))).to_string())
                        }
                    }

                    div(class="settings_buttons") {
                        button(type="button", on:click=delete_index, disabled=*is_indexing.get()) { (get_translation("clear_index", None)) }
                        button(type="submit", disabled=*is_indexing.get()) { (get_translation("index", None)) }
                    }
                }
            }
        }
    }
}
