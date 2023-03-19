use chrono::Local;
use common_lib::{
    actions::OpenPathArgs,
    elasticsearch::{
        AudioChannelType, DocumentData, FileMetadata, ImageData, MultimediaData, ResolutionUnit,
    },
    search::{
        DocumentHighlightedFields, ImageHighlightedFields, MultimediaHighlightedFields,
        SearchResult,
    },
};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use wasm_bindgen::JsValue;

use crate::{
    app::{fetch_empty, widgets::StatusDialogState},
    formatting::{duration_str_from_seconds, file_size_str},
};

use super::{get_local_file_url, PreviewData};

async fn open_path(args: &OpenPathArgs) -> Result<(), JsValue> {
    fetch_empty("/open_path", "POST", Some(args)).await
}

#[component(inline_props)]
pub(super) fn SearchResults<'a, G: Html>(
    cx: Scope<'a>,
    search_results: &'a ReadSignal<Vec<SearchResult>>,
    preview_data: &'a Signal<PreviewData>,
    status_dialog_state: &'a Signal<StatusDialogState>,
) -> View<G> {
    view! { cx,
        Keyed(
            iterable=search_results,
            key=|item| item.id,
            view=move |cx, item| {
                let file_name = item.file.path.file_name().unwrap().to_string_lossy().into_owned();
                let path = item.file.path.clone();
                let path_ = item.file.path.clone();
                let path__ = item.file.path.clone();
                let content_type = item.file.content_type.clone();

                let empty_file = item.file.size == 0;

                let highlighted_path = "Полный путь: ".to_owned() + &item.highlights.path;
                let highlighted_hash = item.highlights.hash.as_ref().map(|x| "Хеш SHA-256: ".to_owned() + x);

                let show_preview = move |_| {
                    preview_data.set(PreviewData {
                        display: true,
                        path: item.file.path.clone(),
                        content_type: content_type.clone(),
                        id: item.file._id.clone().unwrap(),
                    });
                };
                let open_path = move |path| {
                    spawn_local_scoped(cx, async move {
                        status_dialog_state.set(StatusDialogState::Loading);

                        if let Err(e) = open_path(&OpenPathArgs { path }).await {
                            status_dialog_state.set(StatusDialogState::Error(format!(
                                "❌ Ошибка открытия: {e:#?}",
                            )));
                            return;
                        }
                        status_dialog_state.set(StatusDialogState::None);
                    })
                };
                let open_file = move |_| {
                    let path = path_.clone();
                    open_path(path)
                };
                let open_folder = move |_| {
                    let path = path__.parent().unwrap().to_path_buf();
                    open_path(path)
                };

                view! { cx,
                    article(class="search_result") {
                        (if item.file.content_type.starts_with("image")
                                || item.file.content_type.starts_with("video")
                                || item.file.content_type.starts_with("audio") {
                            let img_url = get_local_file_url(&path, Some(&item.file.content_type), true);
                            view! { cx,
                                img(src=(img_url), onerror="this.style.display='none'") {}
                            }
                        } else {
                            view! { cx, }
                        })

                        h3(style="overflow-wrap: anywhere;") { (file_name) }
                        p(style="overflow-wrap: anywhere;", dangerously_set_inner_html=&highlighted_path)
                        div {
                            button(form="search", type="button", disabled=empty_file,
                                on:click=show_preview) { "Показать" }
                            button(form="search", type="button", on:click=open_file) { "Открыть" }
                            button(form="search", type="button", on:click=open_folder) { "Открыть папку" }
                        }
                        (if let Some(content) = item.highlights.content.clone() {
                            view! { cx,
                                p(style="overflow-wrap: anywhere;", dangerously_set_inner_html=&content)
                            }
                        } else {
                            view! { cx, }
                        })
                        (if let Some(summary) = item.highlights.summary.clone() {
                            view! { cx,
                                p(style="overflow-wrap: anywhere;") { (summary) }
                            }
                        } else {
                            view! { cx, }
                        })

                        details {
                            summary { "Основные свойства файла" }

                            p { "Изменено: " (item.file.modified.with_timezone(&Local)) }
                            p { "Размер: " (file_size_str(item.file.size)) }
                            (if let Some(highlighted_hash) = highlighted_hash.clone() {
                                view! { cx,
                                    p(style="overflow-wrap: anywhere;", dangerously_set_inner_html=&highlighted_hash)
                                }
                            } else {
                                view! { cx, }
                            })
                        }

                        (if item.file.image_data.any_metadata() {
                            let image_data = item.file.image_data.clone();
                            let image_highlights = item.highlights.image_data.clone();
                            view! { cx, ImageDataDetails(data=image_data, highlights=image_highlights) }
                        } else {
                            view! { cx, }
                        })

                        (if item.file.multimedia_data.any_metadata() {
                            let multimedia_data = item.file.multimedia_data.clone();
                            let multimedia_highlights = item.highlights.multimedia_data.clone();
                            view! { cx, MultimediaDataDetails(data=multimedia_data, highlights=multimedia_highlights) }
                        } else {
                            view! { cx, }
                        })

                        (if item.file.document_data.any_metadata() {
                            let document_data = item.file.document_data.clone();
                            let document_highlights = item.highlights.document_data.clone();
                            view! { cx, DocumentDataDetails(data=document_data, highlights=document_highlights) }
                        } else {
                            view! { cx, }
                        })
                    }
                }
            }
        )
    }
}

#[component(inline_props)]
fn ImageDataDetails<'a, G: Html>(
    cx: Scope<'a>,
    data: ImageData,
    highlights: ImageHighlightedFields,
) -> View<G> {
    let highlighted_image_make = highlights
        .image_make
        .as_ref()
        .map(|x| "Производитель устройства: ".to_owned() + x);
    let highlighted_image_model = highlights
        .image_model
        .as_ref()
        .map(|x| "Модель устройства: ".to_owned() + x);
    let highlighted_image_software = highlights
        .image_software
        .as_ref()
        .map(|x| "Программное обеспечение: ".to_owned() + x);

    view! { cx,
        details {
            summary { "Свойства изображения" }

            (if let Some(width) = data.width {
                view! { cx, p { "Ширина: " (width) " пикселей" } }
            } else {
                view! { cx, }
            })
            (if let Some(height) = data.height {
                view! { cx, p { "Высота: " (height) " пикселей" } }
            } else {
                view! { cx, }
            })
            (if let (Some(resolution_unit), Some(x_resolution), Some(y_resolution)) =
                    (data.resolution_unit, data.x_resolution, data.y_resolution) {
                let resolution_unit_str = match resolution_unit {
                    ResolutionUnit::Inch => "пикселей на дюйм",
                    ResolutionUnit::Cm => "пикселей на см",
                };
                view! { cx, p { "Разрешение: " (x_resolution) ", " (y_resolution) " " (resolution_unit_str) }}
            } else {
                view! { cx, }
            })
            (if let Some(f_number) = data.f_number {
                view! { cx, p { "F-число: " (f_number) } }
            } else {
                view! { cx, }
            })
            (if let Some(focal_length) = data.focal_length {
                view! { cx, p { "Фокусное расстояние: " (focal_length) " мм" } }
            } else {
                view! { cx, }
            })
            (if let Some(exposure_time) = data.exposure_time {
                view! { cx, p { "Выдержка: " (exposure_time) " с" } }
            } else {
                view! { cx, }
            })
            (if let Some(flash_fired) = data.flash_fired {
                let flash_fired_str = if flash_fired { "да" } else { "нет" };
                view! { cx, p { "Вспышка: " (flash_fired_str) } }
            } else {
                view! { cx, }
            })
            (if let Some(image_make) = highlighted_image_make.clone() {
                view! { cx, p(dangerously_set_inner_html=&image_make) }
            } else {
                view! { cx, }
            })
            (if let Some(image_model) = highlighted_image_model.clone() {
                view! { cx, p(dangerously_set_inner_html=&image_model) }
            } else {
                view! { cx, }
            })
            (if let Some(image_software) = highlighted_image_software.clone() {
                view! { cx, p(dangerously_set_inner_html=&image_software) }
            } else {
                view! { cx, }
            })
        }
    }
}

#[component(inline_props)]
fn MultimediaDataDetails<'a, G: Html>(
    cx: Scope<'a>,
    data: MultimediaData,
    highlights: MultimediaHighlightedFields,
) -> View<G> {
    let highlighted_artist = highlights
        .artist
        .as_ref()
        .map(|x| "Исполнитель: ".to_owned() + x);
    let highlighted_album = highlights.album.as_ref().map(|x| "Альбом: ".to_owned() + x);
    let highlighted_genre = highlights.genre.as_ref().map(|x| "Жанр: ".to_owned() + x);
    let highlighted_track_number = highlights
        .track_number
        .as_ref()
        .map(|x| "Номер трека: ".to_owned() + x);
    let highlighted_disc_number = highlights
        .disc_number
        .as_ref()
        .map(|x| "Номер диска: ".to_owned() + x);
    let highlighted_release_date = highlights
        .release_date
        .as_ref()
        .map(|x| "Дата выпуска: ".to_owned() + x);

    view! { cx,
        details {
            summary { "Свойства мультимедиа" }

            (if let Some(artist) = highlighted_artist.clone() {
                view! { cx, p(dangerously_set_inner_html=&artist) }
            } else {
                view! { cx, }
            })
            (if let Some(album) = highlighted_album.clone() {
                view! { cx, p(dangerously_set_inner_html=&album) }
            } else {
                view! { cx, }
            })
            (if let Some(genre) = highlighted_genre.clone() {
                view! { cx, p(dangerously_set_inner_html=&genre) }
            } else {
                view! { cx, }
            })
            (if let Some(track_number) = highlighted_track_number.clone() {
                view! { cx, p(dangerously_set_inner_html=&track_number) }
            } else {
                view! { cx, }
            })
            (if let Some(disc_number) = highlighted_disc_number.clone() {
                view! { cx, p(dangerously_set_inner_html=&disc_number) }
            } else {
                view! { cx, }
            })
            (if let Some(release_date) = highlighted_release_date.clone() {
                view! { cx, p(dangerously_set_inner_html=&release_date) }
            } else {
                view! { cx, }
            })
            (if let Some(duration) = data.duration {
                let duration_str = duration_str_from_seconds(duration);
                view! { cx, p { "Длительность: " (duration_str) } }
            } else {
                view! { cx, }
            })
            (if let Some(audio_sample_rate) = data.audio_sample_rate {
                view! { cx, p { "Частота дискретизации аудио: " (audio_sample_rate) } }
            } else {
                view! { cx, }
            })
            (if let Some(audio_channel_type) = data.audio_channel_type {
                let audio_channel_type_str = match audio_channel_type {
                    AudioChannelType::Mono => "моно",
                    AudioChannelType::Stereo => "стерео",
                    AudioChannelType::_5_1 => "5.1",
                    AudioChannelType::_7_1 => "7.1",
                    AudioChannelType::_16 => "16 каналов",
                    AudioChannelType::Other => "неизвестно",
                };
                view! { cx, p { "Тип аудиоканала: " (audio_channel_type_str) } }
            } else {
                view! { cx, }
            })
        }
    }
}

#[component(inline_props)]
fn DocumentDataDetails<'a, G: Html>(
    cx: Scope<'a>,
    data: DocumentData,
    highlights: DocumentHighlightedFields,
) -> View<G> {
    let highlighted_title = highlights
        .title
        .as_ref()
        .map(|x| "Заголовок: ".to_owned() + x);
    let highlighted_creator = highlights
        .creator
        .as_ref()
        .map(|x| "Создатель: ".to_owned() + x);

    view! { cx,
        details {
            summary { "Свойства документа" }

            (if let Some(title) = highlighted_title.clone() {
                view! { cx, p(dangerously_set_inner_html=&title) }
            } else {
                view! { cx, }
            })
            (if let Some(creator) = highlighted_creator.clone() {
                view! { cx, p(dangerously_set_inner_html=&creator) }
            } else {
                view! { cx, }
            })
            (if let Some(doc_created) = data.doc_created {
                view! { cx, p { "Создано: " (doc_created.with_timezone(&Local)) } }
            } else {
                view! { cx, }
            })
            (if let Some(doc_modified) = data.doc_modified {
                view! { cx, p { "Изменено: " (doc_modified.with_timezone(&Local)) } }
            } else {
                view! { cx, }
            })
            (if let Some(num_pages) = data.num_pages {
                view! { cx, p { "Страниц: " (num_pages) } }
            } else {
                view! { cx, }
            })
            (if let Some(num_words) = data.num_words {
                view! { cx, p { "Слов: " (num_words) } }
            } else {
                view! { cx, }
            })
            (if let Some(num_characters) = data.num_characters {
                view! { cx, p { "Символов: " (num_characters) } }
            } else {
                view! { cx, }
            })
        }
    }
}
