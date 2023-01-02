use chrono::Local;
use common_lib::{
    actions::OpenPathArgs,
    elasticsearch::{FileES, FileMetadata},
};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use wasm_bindgen::JsValue;

use crate::app::{fetch_empty, widgets::StatusDialogState};

use super::{get_local_file_url, PreviewData};

async fn open_path(args: &OpenPathArgs) -> Result<(), JsValue> {
    fetch_empty("/open_path", "POST", Some(args)).await
}

#[component(inline_props)]
pub(super) fn SearchResults<'a, G: Html>(
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
