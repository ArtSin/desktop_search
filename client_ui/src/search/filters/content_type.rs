use std::collections::HashMap;

use common_lib::search::ContentTypeRequestItem;
use sycamore::prelude::*;
use uuid::Uuid;

use super::CheckboxFilter;

#[derive(Debug, Clone)]
pub struct ContentTypeItem<'a> {
    pub text: &'static str,
    pub type_: &'static str,
    pub enabled: &'a Signal<bool>,
    pub indeterminate: &'a Signal<bool>,
    pub subtypes: &'a Signal<Vec<ContentTypeSubitem<'a>>>,
    pub id: Uuid,
}

impl PartialEq for ContentTypeItem<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ContentTypeItem<'_> {}

impl<'a> ContentTypeItem<'a> {
    fn new(
        cx: Scope<'a>,
        text: &'static str,
        type_: &'static str,
        subtypes: Vec<ContentTypeSubitem<'a>>,
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
pub struct ContentTypeSubitem<'a> {
    pub text: &'static str,
    pub essence: Vec<&'static str>,
    pub enabled: &'a Signal<bool>,
    pub id: Uuid,
}

impl PartialEq for ContentTypeSubitem<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ContentTypeSubitem<'_> {}

impl<'a> ContentTypeSubitem<'a> {
    fn new(cx: Scope<'a>, text: &'static str, essence: Vec<&'static str>) -> Self {
        Self {
            text,
            essence,
            enabled: create_signal(cx, true),
            id: Uuid::new_v4(),
        }
    }
}

#[derive(Prop)]
pub struct ContentTypeFilterProps<'a> {
    pub items: &'a ReadSignal<Vec<ContentTypeItem<'a>>>,
    pub disabled: &'a Signal<bool>,
}

pub fn content_type_filter_items(cx: Scope) -> &Signal<Vec<ContentTypeItem<'_>>> {
    create_signal(
        cx,
        vec![
            ContentTypeItem::new(
                cx,
                "Текстовые форматы",
                "text",
                vec![
                    ContentTypeSubitem::new(cx, "Простой текст", vec!["text/plain"]),
                    ContentTypeSubitem::new(cx, "CSV", vec!["text/csv"]),
                    ContentTypeSubitem::new(cx, "HTML", vec!["text/html"]),
                    ContentTypeSubitem::new(cx, "CSS", vec!["text/css"]),
                    ContentTypeSubitem::new(cx, "Другие", Vec::new()),
                ],
            ),
            ContentTypeItem::new(
                cx,
                "Изображения",
                "image",
                vec![
                    ContentTypeSubitem::new(cx, "JPEG", vec!["image/jpeg"]),
                    ContentTypeSubitem::new(cx, "PNG", vec!["image/png"]),
                    ContentTypeSubitem::new(cx, "GIF", vec!["image/gif"]),
                    ContentTypeSubitem::new(cx, "BMP", vec!["image/bmp"]),
                    ContentTypeSubitem::new(cx, "TIFF", vec!["image/tiff"]),
                    ContentTypeSubitem::new(cx, "SVG", vec!["image/svg+xml"]),
                    ContentTypeSubitem::new(cx, "WebP", vec!["image/webp"]),
                    ContentTypeSubitem::new(
                        cx,
                        "HEIF/HEIC",
                        vec![
                            "image/heif",
                            "image/heic",
                            "image/heif-sequence",
                            "image/heic-sequence",
                        ],
                    ),
                    ContentTypeSubitem::new(cx, "Другие", Vec::new()),
                ],
            ),
            ContentTypeItem::new(
                cx,
                "Аудио",
                "audio",
                vec![
                    ContentTypeSubitem::new(cx, "MP3", vec!["audio/mpeg"]),
                    ContentTypeSubitem::new(cx, "MP4 (аудио)", vec!["audio/mp4"]),
                    ContentTypeSubitem::new(cx, "FLAC", vec!["audio/x-oggflac", "audio/x-flac"]),
                    ContentTypeSubitem::new(cx, "OGG", vec!["audio/ogg", "audio/x-oggpcm"]),
                    ContentTypeSubitem::new(cx, "Opus", vec!["audio/opus"]),
                    ContentTypeSubitem::new(cx, "Vorbis", vec!["audio/vorbis"]),
                    ContentTypeSubitem::new(cx, "MIDI", vec!["audio/midi"]),
                    ContentTypeSubitem::new(cx, "WAV", vec!["audio/vnd.wave", "audio/x-wav"]),
                    ContentTypeSubitem::new(cx, "Другие", Vec::new()),
                ],
            ),
            ContentTypeItem::new(
                cx,
                "Видео",
                "video",
                vec![
                    ContentTypeSubitem::new(cx, "MP4", vec!["video/mp4", "video/x-m4v"]),
                    ContentTypeSubitem::new(cx, "3GPP(2)", vec!["video/3gpp", "video/3gpp2"]),
                    ContentTypeSubitem::new(cx, "QuickTime", vec!["video/quicktime"]),
                    ContentTypeSubitem::new(cx, "FLV (Flash)", vec!["video/x-flv"]),
                    ContentTypeSubitem::new(cx, "Daala", vec!["video/daala"]),
                    ContentTypeSubitem::new(
                        cx,
                        "OGG",
                        vec![
                            "video/x-ogguvs",
                            "video/x-ogm",
                            "video/ogg",
                            "video/x-oggrgb",
                            "video/x-oggyuv",
                        ],
                    ),
                    ContentTypeSubitem::new(cx, "Theora", vec!["video/theora"]),
                    ContentTypeSubitem::new(cx, "Другие", Vec::new()),
                ],
            ),
            ContentTypeItem::new(
                cx,
                "Другие",
                "application",
                vec![
                    ContentTypeSubitem::new(cx, "PDF", vec!["application/pdf"]),
                    ContentTypeSubitem::new(
                        cx,
                        "Microsoft Word (до 2007)",
                        vec!["application/msword"],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        "Microsoft Word (с 2007)",
                        vec![
                            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                            "application/vnd.openxmlformats-officedocument.wordprocessingml.template",
                            "application/vnd.ms-word.document.macroEnabled.12",
                            "application/vnd.ms-word.template.macroEnabled.12"
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        "Microsoft Excel (до 2007)",
                        vec!["application/vnd.ms-excel"],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        "Microsoft Excel (с 2007)",
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
                        "Microsoft PowerPoint (до 2007)",
                        vec!["application/vnd.ms-powerpoint"],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        "Microsoft PowerPoint (с 2007)",
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
                        "OpenDocument Text",
                        vec![
                            "application/vnd.oasis.opendocument.text",
                            "application/vnd.oasis.opendocument.text-template",
                            "application/vnd.oasis.opendocument.text-master",
                            "application/vnd.oasis.opendocument.flat.text",
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        "OpenDocument Spreadsheet",
                        vec![
                            "application/vnd.oasis.opendocument.spreadsheet",
                            "application/vnd.oasis.opendocument.spreadsheet-template",
                            "application/vnd.oasis.opendocument.flat.spreadsheet",
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        "OpenDocument Presentation",
                        vec![
                            "application/vnd.oasis.opendocument.presentation",
                            "application/vnd.oasis.opendocument.presentation-template",
                            "application/vnd.oasis.opendocument.flat.presentation",
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        "Apple Pages",
                        vec![
                            "application/vnd.apple.pages",
                            "application/vnd.apple.pages.13",
                            "application/vnd.apple.pages.18",
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        "Apple Numbers",
                        vec![
                            "application/vnd.apple.numbers",
                            "application/vnd.apple.numbers.13",
                            "application/vnd.apple.numbers.18",
                        ],
                    ),
                    ContentTypeSubitem::new(
                        cx,
                        "Apple Keynote",
                        vec![
                            "application/vnd.apple.keynote",
                            "application/vnd.apple.keynote.13",
                            "application/vnd.apple.keynote.18",
                        ],
                    ),
                    ContentTypeSubitem::new(cx, "ZIP", vec!["application/zip"]),
                    ContentTypeSubitem::new(cx, "RAR", vec!["application/x-rar-compressed"]),
                    ContentTypeSubitem::new(cx, "7-Zip", vec!["application/x-7z-compressed"]),
                    ContentTypeSubitem::new(cx, "Gzip", vec!["application/gzip"]),
                    ContentTypeSubitem::new(cx, "Zlib", vec!["application/zlib"]),
                    ContentTypeSubitem::new(cx, "Другие", Vec::new()),
                ],
            ),
        ],
    )
}

pub fn get_content_type_request_items<'a>(
    items: &'a ReadSignal<Vec<ContentTypeItem<'a>>>,
) -> Vec<ContentTypeRequestItem> {
    items
        .get()
        .iter()
        .map(|item| {
            let get_subtypes = |item: &ContentTypeItem, enabled: bool| {
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

pub fn load_from_content_type_request_items<'a>(
    request_items: &[ContentTypeRequestItem],
    items: &'a Signal<Vec<ContentTypeItem<'a>>>,
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
pub fn ContentTypeFilter<'a, G: Html>(cx: Scope<'a>, props: ContentTypeFilterProps<'a>) -> View<G> {
    view! { cx,
        fieldset {
            legend { "Тип файла" }

            CheckboxFilter(text="Любой", id="content_type_disabled", value_enabled=props.disabled)

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
                                        label(for=item.id) { (item.text) }
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
                                                    label(for=subitem.id) { (subitem.text) }
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
