use common_lib::{status::IndexStats, IndexingStatus};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use wasm_bindgen::JsValue;

use crate::app::{
    fetch, fetch_empty,
    widgets::{StatusDialogState, StatusMessage},
};

async fn get_indexing_status() -> Result<IndexingStatus, JsValue> {
    fetch("/index", "GET", None::<&()>).await
}

async fn get_index_stats() -> Result<IndexStats, JsValue> {
    fetch("/index_stats", "GET", None::<&()>).await
}

async fn index() -> Result<(), JsValue> {
    fetch_empty("/index", "PATCH", None::<&()>).await
}

#[component(inline_props)]
pub fn Status<'a, G: Html>(
    cx: Scope<'a>,
    status_dialog_state: &'a Signal<StatusDialogState>,
) -> View<G> {
    let indexing_status = create_signal(cx, IndexingStatus::Finished);
    let index_stats = create_signal(cx, IndexStats::default());

    let indexing_status_str = create_memo(cx, || indexing_status.get().to_string());
    let is_indexing = create_memo(cx, || !indexing_status.get().can_start());

    let update = move || {
        spawn_local_scoped(cx, async move {
            status_dialog_state.set(StatusDialogState::Loading);

            match get_indexing_status().await {
                Ok(res) => {
                    indexing_status.set(res);
                }
                Err(e) => {
                    status_dialog_state.set(StatusDialogState::Error(format!(
                        "❌ Ошибка загрузки статуса индексирования: {:#?}",
                        e
                    )));
                    return;
                }
            }

            match get_index_stats().await {
                Ok(res) => {
                    index_stats.set(res);
                    status_dialog_state.set(StatusDialogState::None);
                }
                Err(e) => {
                    status_dialog_state.set(StatusDialogState::Error(format!(
                        "❌ Ошибка загрузки статистики: {:#?}",
                        e
                    )));
                }
            }
        })
    };

    update();

    let index = move |_| {
        spawn_local_scoped(cx, async move {
            status_dialog_state.set(StatusDialogState::Loading);

            if let Err(e) = index().await {
                status_dialog_state.set(StatusDialogState::Error(format!(
                    "❌ Ошибка индексирования: {:#?}",
                    e,
                )));
                return;
            }

            update();
        })
    };

    view! { cx,
        div(class="main_container") {
            main {
                StatusMessage(status_str=indexing_status_str)

                form(id="status", on:submit=index, action="javascript:void(0);") {
                    fieldset {
                        legend { "Статистика" }
                        p {
                            "Количество файлов в индексе: " (index_stats.get().doc_cnt)
                        }
                        p {
                            "Размер индекса (МиБ): "
                            (format!("{:.4}", (index_stats.get().index_size as f64) / 1024.0 / 1024.0))
                        }
                    }

                    div(class="settings_buttons") {
                        button(type="button", on:click=move |_| update()) { "Обновить статус" }
                        button(type="submit", disabled=*is_indexing.get()) { "Индексировать" }
                    }
                }
            }
        }
    }
}
