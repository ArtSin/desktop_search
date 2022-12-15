use common_lib::status::IndexStats;
use serde_wasm_bindgen::from_value;
use sycamore::{futures::spawn_local_scoped, prelude::*};

use crate::app::{invoke, StatusMessage};

#[component]
pub fn Status<G: Html>(cx: Scope) -> View<G> {
    let status_str = create_signal(cx, String::new());
    let index_stats = create_signal(cx, IndexStats::default());

    let load_stats = move || {
        spawn_local_scoped(cx, async move {
            status_str.set("⏳ Загрузка...".to_owned());

            match invoke("get_index_stats", wasm_bindgen::JsValue::UNDEFINED)
                .await
                .map_err(|e| e.as_string().unwrap())
                .and_then(|x| from_value::<IndexStats>(x).map_err(|e| e.to_string()))
            {
                Ok(x) => {
                    index_stats.set(x);
                    status_str.set("".to_owned());
                }
                Err(e) => {
                    status_str.set("❌ Ошибка загрузки статистики: ".to_owned() + &e);
                }
            }
        })
    };

    load_stats();

    view! { cx,
        div(class="main_container") {
            main {
                StatusMessage(status_str=status_str)

                form(id="status", action="javascript:void(0);") {
                    fieldset {
                        legend { "Статистика" }
                        "Количество файлов в индексе: " (index_stats.get().doc_cnt)
                    }
                }
            }
        }
    }
}
