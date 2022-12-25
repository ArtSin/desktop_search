use common_lib::{status::IndexStats, IndexingStatus};
use serde_wasm_bindgen::from_value;
use sycamore::{futures::spawn_local_scoped, prelude::*};

use crate::app::{
    invoke,
    widgets::{StatusDialogState, StatusMessage},
};

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

            match invoke("get_indexing_status", wasm_bindgen::JsValue::UNDEFINED)
                .await
                .map_err(|e| e.as_string().unwrap())
                .and_then(|x| from_value::<IndexingStatus>(x).map_err(|e| e.to_string()))
            {
                Ok(x) => {
                    indexing_status.set(x);
                    status_dialog_state.set(StatusDialogState::None);
                }
                Err(e) => {
                    status_dialog_state.set(StatusDialogState::Error(
                        "❌ Ошибка загрузки статуса индексирования: ".to_owned() + &e,
                    ));
                    return;
                }
            }

            match invoke("get_index_stats", wasm_bindgen::JsValue::UNDEFINED)
                .await
                .map_err(|e| e.as_string().unwrap())
                .and_then(|x| from_value::<IndexStats>(x).map_err(|e| e.to_string()))
            {
                Ok(x) => {
                    index_stats.set(x);
                    status_dialog_state.set(StatusDialogState::None);
                }
                Err(e) => {
                    status_dialog_state.set(StatusDialogState::Error(
                        "❌ Ошибка загрузки статистики: ".to_owned() + &e,
                    ));
                }
            }
        })
    };

    update();

    let index = move |_| {
        spawn_local_scoped(cx, async move {
            status_dialog_state.set(StatusDialogState::Loading);

            if let Err(e) = invoke("index", wasm_bindgen::JsValue::UNDEFINED)
                .await
                .map_err(|e| e.as_string().unwrap())
            {
                status_dialog_state.set(StatusDialogState::Error(
                    "❌ Ошибка индексирования: ".to_owned() + &e,
                ));
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
