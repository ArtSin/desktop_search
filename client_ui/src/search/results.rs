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
use fluent_bundle::FluentArgs;
use sycamore::{futures::spawn_local_scoped, prelude::*};
use wasm_bindgen::JsValue;

use crate::{
    app::{fetch_empty, get_translation, widgets::StatusDialogState},
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

                let highlighted_path_args = FluentArgs::from_iter([("path", item.highlights.path)]);
                let highlighted_path = get_translation("results_path", Some(&highlighted_path_args)).to_string();
                let highlighted_hash = item.highlights.hash.map(|x| {
                    let highlighted_hash_args = FluentArgs::from_iter([("hash", x)]);
                    get_translation("results_hash", Some(&highlighted_hash_args)).to_string()
                });

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
                            let error_args = FluentArgs::from_iter([("error", format!("{e:#?}"))]);
                            let error_str = get_translation("opening_error", Some(&error_args)).to_string();
                            status_dialog_state.set(StatusDialogState::Error(error_str));
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
                                on:click=show_preview) { (get_translation("show", None)) }
                            button(form="search", type="button", on:click=open_file) { (get_translation("open", None)) }
                            button(form="search", type="button", on:click=open_folder) { (get_translation("open_folder", None)) }
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
                            summary { (get_translation("main_file_properties", None)) }

                            p {
                                (get_translation("results_modified", Some(&FluentArgs::from_iter(
                                    [("modified", item.file.modified.with_timezone(&Local).to_string())]))).to_string())
                            }
                            p {
                                (get_translation("results_size", Some(&FluentArgs::from_iter(
                                    [("size", file_size_str(item.file.size))]))).to_string())
                            }
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
    let highlighted_image_make = highlights.image_make.map(|x| {
        let highlighted_image_make_args = FluentArgs::from_iter([("device_manufacturer", x)]);
        get_translation(
            "results_device_manufacturer",
            Some(&highlighted_image_make_args),
        )
        .to_string()
    });
    let highlighted_image_model = highlights.image_model.map(|x| {
        let highlighted_image_model_args = FluentArgs::from_iter([("device_model", x)]);
        get_translation("results_device_model", Some(&highlighted_image_model_args)).to_string()
    });
    let highlighted_image_software = highlights.image_software.map(|x| {
        let highlighted_image_software_args = FluentArgs::from_iter([("image_software", x)]);
        get_translation(
            "results_image_software",
            Some(&highlighted_image_software_args),
        )
        .to_string()
    });

    view! { cx,
        details {
            summary { (get_translation("image_properties", None)) }

            (if let Some(width) = data.width {
                view! { cx,
                    p { (get_translation("results_width", Some(&FluentArgs::from_iter(
                            [("width", width)]))).to_string()) }
                }
            } else {
                view! { cx, }
            })
            (if let Some(height) = data.height {
                view! { cx,
                    p { (get_translation("results_height", Some(&FluentArgs::from_iter(
                            [("height", height)]))).to_string()) }
                }
            } else {
                view! { cx, }
            })
            (if let (Some(resolution_unit), Some(x_resolution), Some(y_resolution)) =
                    (data.resolution_unit, data.x_resolution, data.y_resolution) {
                let resolution_unit_str = match resolution_unit {
                    ResolutionUnit::Inch => get_translation("pixels_per_inch", None),
                    ResolutionUnit::Cm => get_translation("pixels_per_cm", None),
                };
                view! { cx,
                    p { (get_translation("results_resolution", Some(&FluentArgs::from_iter(
                            [("x_resolution", x_resolution.to_string()),
                            ("y_resolution", y_resolution.to_string()),
                            ("resolution_unit", resolution_unit_str.to_string())]))).to_string()) }
                }
            } else {
                view! { cx, }
            })
            (if let Some(f_number) = data.f_number {
                view! { cx,
                    p { (get_translation("results_f_number", Some(&FluentArgs::from_iter(
                            [("f_number", f_number.to_string())]))).to_string()) }
                }
            } else {
                view! { cx, }
            })
            (if let Some(focal_length) = data.focal_length {
                view! { cx,
                    p { (get_translation("results_focal_length", Some(&FluentArgs::from_iter(
                            [("focal_length", focal_length.to_string())]))).to_string()) }
                }
            } else {
                view! { cx, }
            })
            (if let Some(exposure_time) = data.exposure_time {
                view! { cx,
                    p { (get_translation("results_exposure_time", Some(&FluentArgs::from_iter(
                            [("exposure_time", exposure_time.to_string())]))).to_string()) }
                }
            } else {
                view! { cx, }
            })
            (if let Some(flash_fired) = data.flash_fired {
                let flash_fired_str = get_translation(if flash_fired { "yes" } else { "no" }, None);
                view! { cx,
                    p { (get_translation("results_flash", Some(&FluentArgs::from_iter(
                            [("flash", flash_fired_str.as_ref())]))).to_string()) }
                }
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
    let highlighted_artist = highlights.artist.map(|x| {
        let highlighted_artist_args = FluentArgs::from_iter([("artist", x)]);
        get_translation("results_artist", Some(&highlighted_artist_args)).to_string()
    });
    let highlighted_album = highlights.album.map(|x| {
        let highlighted_album_args = FluentArgs::from_iter([("album", x)]);
        get_translation("results_album", Some(&highlighted_album_args)).to_string()
    });
    let highlighted_genre = highlights.genre.map(|x| {
        let highlighted_genre_args = FluentArgs::from_iter([("genre", x)]);
        get_translation("results_genre", Some(&highlighted_genre_args)).to_string()
    });
    let highlighted_track_number = highlights.track_number.map(|x| {
        let highlighted_track_number_args = FluentArgs::from_iter([("track_number", x)]);
        get_translation("results_track_number", Some(&highlighted_track_number_args)).to_string()
    });
    let highlighted_disc_number = highlights.disc_number.map(|x| {
        let highlighted_disc_number_args = FluentArgs::from_iter([("disc_number", x)]);
        get_translation("results_disc_number", Some(&highlighted_disc_number_args)).to_string()
    });
    let highlighted_release_date = highlights.release_date.map(|x| {
        let highlighted_release_date_args = FluentArgs::from_iter([("release_date", x)]);
        get_translation("results_release_date", Some(&highlighted_release_date_args)).to_string()
    });

    view! { cx,
        details {
            summary { (get_translation("multimedia_properties", None)) }

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
                view! { cx,
                    p { (get_translation("results_duration", Some(&FluentArgs::from_iter(
                            [("duration", AsRef::<str>::as_ref(&duration_str))]))).to_string()) }
                }
            } else {
                view! { cx, }
            })
            (if let Some(audio_sample_rate) = data.audio_sample_rate {
                view! { cx,
                    p { (get_translation("results_audio_sample_rate", Some(&FluentArgs::from_iter(
                            [("audio_sample_rate", audio_sample_rate)]))).to_string()) }
                }
            } else {
                view! { cx, }
            })
            (if let Some(audio_channel_type) = data.audio_channel_type {
                let audio_channel_type_str = match audio_channel_type {
                    AudioChannelType::Mono => get_translation("audio_mono", None),
                    AudioChannelType::Stereo => get_translation("audio_stereo", None),
                    AudioChannelType::_5_1 => get_translation("audio_5_1", None),
                    AudioChannelType::_7_1 => get_translation("audio_7_1", None),
                    AudioChannelType::_16 => get_translation("audio_16", None),
                    AudioChannelType::Other => get_translation("audio_other", None),
                };
                view! { cx,
                    p { (get_translation("results_audio_channel_type", Some(&FluentArgs::from_iter(
                            [("audio_channel_type", audio_channel_type_str.as_ref())]))).to_string()) }
                }
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
    let highlighted_title = highlights.title.map(|x| {
        let highlighted_title_args = FluentArgs::from_iter([("title", x)]);
        get_translation("results_title", Some(&highlighted_title_args)).to_string()
    });
    let highlighted_creator = highlights.creator.map(|x| {
        let highlighted_creator_args = FluentArgs::from_iter([("creator", x)]);
        get_translation("results_creator", Some(&highlighted_creator_args)).to_string()
    });

    view! { cx,
        details {
            summary { (get_translation("document_properties", None)) }

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
                view! { cx,
                    p { (get_translation("results_doc_created", Some(&FluentArgs::from_iter(
                            [("doc_created", doc_created.with_timezone(&Local).to_string())]))).to_string()) }
                }
            } else {
                view! { cx, }
            })
            (if let Some(doc_modified) = data.doc_modified {
                view! { cx,
                    p { (get_translation("results_doc_modified", Some(&FluentArgs::from_iter(
                            [("doc_modified", doc_modified.with_timezone(&Local).to_string())]))).to_string()) }
                }
            } else {
                view! { cx, }
            })
            (if let Some(num_pages) = data.num_pages {
                view! { cx,
                    p { (get_translation("results_num_pages", Some(&FluentArgs::from_iter(
                            [("num_pages", num_pages)]))).to_string()) }
                }
            } else {
                view! { cx, }
            })
            (if let Some(num_words) = data.num_words {
                view! { cx,
                    p { (get_translation("results_num_words", Some(&FluentArgs::from_iter(
                            [("num_words", num_words)]))).to_string()) }
                }
            } else {
                view! { cx, }
            })
            (if let Some(num_characters) = data.num_characters {
                view! { cx,
                    p { (get_translation("results_num_characters", Some(&FluentArgs::from_iter(
                            [("num_characters", num_characters)]))).to_string()) }
                }
            } else {
                view! { cx, }
            })
        }
    }
}
