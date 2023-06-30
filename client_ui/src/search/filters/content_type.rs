use std::{borrow::Cow, collections::HashMap, fmt::Display};

use common_lib::search::ContentTypeRequestItem;
use sycamore::prelude::*;
use uuid::Uuid;

use crate::app::get_translation;

use super::CheckboxFilter;

#[derive(Debug, Clone)]
pub struct ContentTypeItem<'a, S: AsRef<str>> {
    pub text: S,
    pub type_: &'static str,
    pub enabled: &'a Signal<bool>,
    pub indeterminate: &'a Signal<bool>,
    pub subtypes: &'a Signal<Vec<ContentTypeSubitem<'a, S>>>,
    pub id: Uuid,
}

impl<S: AsRef<str>> PartialEq for ContentTypeItem<'_, S> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<S: AsRef<str>> Eq for ContentTypeItem<'_, S> {}

impl<'a, S: AsRef<str>> ContentTypeItem<'a, S> {
    fn new(
        cx: Scope<'a>,
        text: S,
        type_: &'static str,
        subtypes: Vec<ContentTypeSubitem<'a, S>>,
    ) -> Self {
        Self {
            text,
            type_,
            enabled: create_signal(cx, true),
            indeterminate: create_signal(cx, false),
            subtypes: create_signal(cx, subtypes),
            id: Uuid::new_v4(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContentTypeSubitem<'a, S: AsRef<str>> {
    pub text: S,
    pub essence: Vec<&'static str>,
    pub enabled: &'a Signal<bool>,
    pub id: Uuid,
}

impl<S: AsRef<str>> PartialEq for ContentTypeSubitem<'_, S> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<S: AsRef<str>> Eq for ContentTypeSubitem<'_, S> {}

impl<'a, S: AsRef<str>> ContentTypeSubitem<'a, S> {
    fn new(cx: Scope<'a>, text: S, essence: Vec<&'static str>) -> Self {
        Self {
            text,
            essence,
            enabled: create_signal(cx, true),
            id: Uuid::new_v4(),
        }
    }
}

#[derive(Prop)]
pub struct ContentTypeFilterProps<'a, S: AsRef<str>> {
    pub items: &'a ReadSignal<Vec<ContentTypeItem<'a, S>>>,
    pub disabled: &'a Signal<bool>,
}

pub fn content_type_filter_items(cx: Scope) -> &Signal<Vec<ContentTypeItem<'_, Cow<'_, str>>>> {
    create_signal(
        cx,
        vec![
            ContentTypeItem::new(
                cx,
                get_translation("mime_text", None),
                "text",
                vec![
                    ContentTypeSubitem::new(cx, get_translation("mime_text_plain", None), vec!["text/plain"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_text_csv", None), vec!["text/csv"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_text_html", None), vec!["text/html"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_text_css", None), vec!["text/css"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_other", None), Vec::new()),
                ],
            ),
            ContentTypeItem::new(
                cx,
                get_translation("mime_image", None),
                "image",
                vec![
                    ContentTypeSubitem::new(cx, get_translation("mime_image_jpeg", None), vec!["image/jpeg"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_image_png", None), vec!["image/png"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_image_gif", None), vec!["image/gif"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_image_bmp", None), vec!["image/bmp"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_image_tiff", None), vec!["image/tiff"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_image_svg", None), vec!["image/svg+xml"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_image_webp", None), vec!["image/webp"]),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_image_heif_heic", None),
                        vec![
                            "image/heif",
                            "image/heic",
                            "image/heif-sequence",
                            "image/heic-sequence",
                        ],
                    ),
                    ContentTypeSubitem::new(cx, get_translation("mime_other", None), Vec::new()),
                ],
            ),
            ContentTypeItem::new(
                cx,
                get_translation("mime_audio", None),
                "audio",
                vec![
                    ContentTypeSubitem::new(cx, get_translation("mime_audio_mp3", None), vec!["audio/mpeg"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_audio_mp4", None), vec!["audio/mp4"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_audio_flac", None), vec!["audio/x-oggflac", "audio/x-flac"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_audio_ogg", None), vec!["audio/ogg", "audio/x-oggpcm"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_audio_opus", None), vec!["audio/opus"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_audio_vorbis", None), vec!["audio/vorbis"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_audio_midi", None), vec!["audio/midi"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_audio_wav", None), vec!["audio/vnd.wave", "audio/x-wav"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_other", None), Vec::new()),
                ],
            ),
            ContentTypeItem::new(
                cx,
                get_translation("mime_video", None),
                "video",
                vec![
                    ContentTypeSubitem::new(cx, get_translation("mime_video_mp4", None), vec!["video/mp4", "video/x-m4v"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_video_3gpp", None), vec!["video/3gpp", "video/3gpp2"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_video_quicktime", None), vec!["video/quicktime"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_video_flv", None), vec!["video/x-flv"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_video_daala", None), vec!["video/daala"]),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_video_ogg", None),
                        vec![
                            "video/x-ogguvs",
                            "video/x-ogm",
                            "video/ogg",
                            "video/x-oggrgb",
                            "video/x-oggyuv",
                        ],
                    ),
                    ContentTypeSubitem::new(cx, get_translation("mime_video_theora", None), vec!["video/theora"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_other", None), Vec::new()),
                ],
            ),
            ContentTypeItem::new(
                cx,
                get_translation("mime_application", None),
                "application",
                vec![
                    ContentTypeSubitem::new(cx, get_translation("mime_application_pdf", None), vec!["application/pdf"]),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_application_word_old", None),
                        vec!["application/msword"],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_application_word_new", None),
                        vec![
                            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                            "application/vnd.openxmlformats-officedocument.wordprocessingml.template",
                            "application/vnd.ms-word.document.macroEnabled.12",
                            "application/vnd.ms-word.template.macroEnabled.12"
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_application_excel_old", None),
                        vec!["application/vnd.ms-excel"],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_application_excel_new", None),
                        vec![
                            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                            "application/vnd.openxmlformats-officedocument.spreadsheetml.template",
                            "application/vnd.ms-excel.sheet.macroEnabled.12",
                            "application/vnd.ms-excel.template.macroEnabled.12",
                            "application/vnd.ms-excel.addin.macroEnabled.12",
                            "application/vnd.ms-excel.sheet.binary.macroEnabled.12"
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_application_powerpoint_old", None),
                        vec!["application/vnd.ms-powerpoint"],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_application_powerpoint_new", None),
                        vec![
                            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                            "application/vnd.openxmlformats-officedocument.presentationml.template",
                            "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
                            "application/vnd.ms-powerpoint.addin.macroEnabled.12",
                            "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
                            "application/vnd.ms-powerpoint.template.macroEnabled.12",
                            "application/vnd.ms-powerpoint.slideshow.macroEnabled.12"
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_application_odt", None),
                        vec![
                            "application/vnd.oasis.opendocument.text",
                            "application/vnd.oasis.opendocument.text-template",
                            "application/vnd.oasis.opendocument.text-master",
                            "application/vnd.oasis.opendocument.flat.text",
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_application_ods", None),
                        vec![
                            "application/vnd.oasis.opendocument.spreadsheet",
                            "application/vnd.oasis.opendocument.spreadsheet-template",
                            "application/vnd.oasis.opendocument.flat.spreadsheet",
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_application_odp", None),
                        vec![
                            "application/vnd.oasis.opendocument.presentation",
                            "application/vnd.oasis.opendocument.presentation-template",
                            "application/vnd.oasis.opendocument.flat.presentation",
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_application_apple_pages", None),
                        vec![
                            "application/vnd.apple.pages",
                            "application/vnd.apple.pages.13",
                            "application/vnd.apple.pages.18",
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_application_apple_numbers", None),
                        vec![
                            "application/vnd.apple.numbers",
                            "application/vnd.apple.numbers.13",
                            "application/vnd.apple.numbers.18",
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        get_translation("mime_application_apple_keynote", None),
                        vec![
                            "application/vnd.apple.keynote",
                            "application/vnd.apple.keynote.13",
                            "application/vnd.apple.keynote.18",
                        ],
                    ),
                    ContentTypeSubitem::new(cx, get_translation("mime_application_zip", None), vec!["application/zip"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_application_rar", None), vec!["application/x-rar-compressed"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_application_7zip", None), vec!["application/x-7z-compressed"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_application_gzip", None), vec!["application/gzip"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_application_zlib", None), vec!["application/zlib"]),
                    ContentTypeSubitem::new(cx, get_translation("mime_other", None), Vec::new()),
                ],
            ),
        ],
    )
}

pub fn get_content_type_request_items<'a, S: AsRef<str>>(
    items: &'a ReadSignal<Vec<ContentTypeItem<'a, S>>>,
) -> Vec<ContentTypeRequestItem> {
    items
        .get()
        .iter()
        .map(|item| {
            let get_subtypes = |item: &ContentTypeItem<S>, enabled: bool| {
                item.subtypes
                    .get()
                    .iter()
                    .filter(|subitem| *subitem.enabled.get() == enabled)
                    .flat_map(|subitem| subitem.essence.iter().map(|x| (*x).to_owned()))
                    .collect()
            };

            let include_other = *item.subtypes.get().last().unwrap().enabled.get();
            match (
                *item.enabled.get(),
                *item.indeterminate.get(),
                include_other,
            ) {
                (true, false, true) => ContentTypeRequestItem::IncludeType {
                    type_: item.type_.to_owned(),
                },
                (false, false, false) => ContentTypeRequestItem::ExcludeType {
                    type_: item.type_.to_owned(),
                },
                (false, true, false) => ContentTypeRequestItem::IncludeSubtypes {
                    subtypes: get_subtypes(item, true),
                },
                (false, true, true) => ContentTypeRequestItem::ExcludeSubtypes {
                    type_: item.type_.to_owned(),
                    subtypes: get_subtypes(item, false),
                },
                _ => unreachable!(),
            }
        })
        .collect()
}

pub fn load_from_content_type_request_items<'a, S: AsRef<str>>(
    request_items: &[ContentTypeRequestItem],
    items: &'a Signal<Vec<ContentTypeItem<'a, S>>>,
) {
    let items_value = items.get();
    let items_hm: HashMap<_, _> = items_value.iter().map(|x| (x.type_, x)).collect();
    let subitems_value: Vec<_> = items_value.iter().map(|x| (x, x.subtypes.get())).collect();
    let subitems_hm: HashMap<_, _> = subitems_value
        .iter()
        .flat_map(|(x, subtypes)| {
            subtypes
                .iter()
                .flat_map(move |y| y.essence.iter().map(move |&e| (e, (*x, y))))
        })
        .collect();

    for req_item in request_items {
        match req_item {
            ContentTypeRequestItem::IncludeType { type_ } => {
                let item = items_hm.get(type_.as_str()).unwrap();
                item.enabled.set(true);
                item.indeterminate.set(false);
                for subitem in item.subtypes.get().iter() {
                    subitem.enabled.set(true);
                }
            }
            ContentTypeRequestItem::ExcludeType { type_ } => {
                let item = items_hm.get(type_.as_str()).unwrap();
                item.enabled.set(false);
                item.indeterminate.set(false);
                for subitem in item.subtypes.get().iter() {
                    subitem.enabled.set(false);
                }
            }
            ContentTypeRequestItem::IncludeSubtypes { subtypes } => {
                let mut is_first = true;
                for subtype in subtypes {
                    let (item, subitem) = subitems_hm.get(subtype.as_str()).unwrap();
                    if is_first {
                        item.enabled.set(false);
                        item.indeterminate.set(true);
                        for subitem in item.subtypes.get().iter() {
                            subitem.enabled.set(false);
                        }
                        is_first = false;
                    }
                    subitem.enabled.set(true);
                }
            }
            ContentTypeRequestItem::ExcludeSubtypes { type_, subtypes } => {
                let item = items_hm.get(type_.as_str()).unwrap();
                item.enabled.set(false);
                item.indeterminate.set(true);
                for subitem in item.subtypes.get().iter() {
                    subitem.enabled.set(true);
                }

                for subtype in subtypes {
                    let (_, subitem) = subitems_hm.get(subtype.as_str()).unwrap();
                    subitem.enabled.set(false);
                }
            }
        }
    }
}

#[component]
pub fn ContentTypeFilter<'a, S: AsRef<str> + Clone + Display, G: Html>(
    cx: Scope<'a>,
    props: ContentTypeFilterProps<'a, S>,
) -> View<G> {
    view! { cx,
        fieldset {
            legend { (get_translation("filter_file_type", None)) }

            CheckboxFilter(text=get_translation("filter_file_type_any", None),
                id="content_type_disabled", value_enabled=props.disabled)

            (if !*props.disabled.get() {
                view! { cx,
                    Keyed(
                        iterable=props.items,
                        key=|item| item.id,
                        view=move |cx, item| {
                            let on_item_click = |_| {
                                item.enabled.set(!*item.enabled.get());
                                item.indeterminate.set(false);
                                for x in item.subtypes.get().iter() {
                                    x.enabled.set(*item.enabled.get());
                                }
                            };

                            view! { cx,
                                details {
                                    summary {
                                        input(type="checkbox", id=item.id, name=item.id, prop:checked=*item.enabled.get(),
                                            prop:indeterminate=*item.indeterminate.get(), on:click=on_item_click) {}
                                        label(for=item.id) { (item.text.to_string()) }
                                    }

                                    Keyed(
                                        iterable=item.subtypes,
                                        key=|subitem| subitem.id,
                                        view=move |cx, subitem| {
                                            let on_subitem_click = |_| {
                                                subitem.enabled.set(!*subitem.enabled.get());

                                                let subitems_none = item.subtypes.get().iter().all(|subitem| !*subitem.enabled.get());
                                                let subitems_all = item.subtypes.get().iter().all(|subitem| *subitem.enabled.get());
                                                match (subitems_none, subitems_all) {
                                                    (true, false) => {
                                                        item.enabled.set(false);
                                                        item.indeterminate.set(false);
                                                    }
                                                    (false, false) => {
                                                        item.enabled.set(false);
                                                        item.indeterminate.set(true);
                                                    }
                                                    (false, true) => {
                                                        item.enabled.set(true);
                                                        item.indeterminate.set(false);
                                                    }
                                                    _ => unreachable!(),
                                                }
                                            };

                                            view! { cx,
                                                div(class="radio_checkbox_field") {
                                                    input(type="checkbox", id=subitem.id, name=subitem.id,
                                                        prop:checked=*subitem.enabled.get(), on:click=on_subitem_click) {}
                                                    label(for=subitem.id) { (subitem.text.to_string()) }
                                                }
                                            }
                                        }
                                    )
                                }
                            }
                        }
                    )
                }
            } else {
                view! { cx, }
            })
        }
    }
}
