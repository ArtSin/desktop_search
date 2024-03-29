use chrono::{DateTime, Utc};
use common_lib::{
    elasticsearch::{AudioChannelType, ResolutionUnit},
    search::{DocumentSearchRequest, ImageSearchRequest, MultimediaSearchRequest},
};
use sycamore::prelude::*;

use crate::app::get_translation;

use super::filters::{
    CheckboxFilter, CheckboxOptionFilter, DateTimeFilter, NumberFilter, SelectFilter,
    SelectOptionFilter,
};

#[derive(Clone)]
pub struct ImageFiltersData<'a> {
    image_make_enabled: &'a Signal<bool>,
    image_model_enabled: &'a Signal<bool>,
    image_software_enabled: &'a Signal<bool>,

    width_from: &'a Signal<Option<u32>>,
    width_to: &'a Signal<Option<u32>>,
    width_valid: &'a Signal<bool>,

    height_from: &'a Signal<Option<u32>>,
    height_to: &'a Signal<Option<u32>>,
    height_valid: &'a Signal<bool>,

    resolution_unit: &'a Signal<ResolutionUnit>,

    x_resolution_from: &'a Signal<Option<f32>>,
    x_resolution_to: &'a Signal<Option<f32>>,
    x_resolution_valid: &'a Signal<bool>,

    y_resolution_from: &'a Signal<Option<f32>>,
    y_resolution_to: &'a Signal<Option<f32>>,
    y_resolution_valid: &'a Signal<bool>,

    f_number_from: &'a Signal<Option<f32>>,
    f_number_to: &'a Signal<Option<f32>>,
    f_number_valid: &'a Signal<bool>,

    focal_length_from: &'a Signal<Option<f32>>,
    focal_length_to: &'a Signal<Option<f32>>,
    focal_length_valid: &'a Signal<bool>,

    exposure_time_from: &'a Signal<Option<f32>>,
    exposure_time_to: &'a Signal<Option<f32>>,
    exposure_time_valid: &'a Signal<bool>,

    flash_fired: &'a Signal<Option<bool>>,

    pub any_invalid: &'a ReadSignal<bool>,
}

impl<'a> ImageFiltersData<'a> {
    pub fn new(cx: Scope<'a>) -> Self {
        let width_valid = create_signal(cx, true);
        let height_valid = create_signal(cx, true);
        let x_resolution_valid = create_signal(cx, true);
        let y_resolution_valid = create_signal(cx, true);
        let f_number_valid = create_signal(cx, true);
        let focal_length_valid = create_signal(cx, true);
        let exposure_time_valid = create_signal(cx, true);
        let any_invalid = create_memo(cx, || {
            !*width_valid.get()
                || !*height_valid.get()
                || !*x_resolution_valid.get()
                || !*y_resolution_valid.get()
                || !*f_number_valid.get()
                || !*focal_length_valid.get()
                || !*exposure_time_valid.get()
        });

        Self {
            image_make_enabled: create_signal(cx, true),
            image_model_enabled: create_signal(cx, true),
            image_software_enabled: create_signal(cx, true),

            width_from: create_signal(cx, None),
            width_to: create_signal(cx, None),
            width_valid,

            height_from: create_signal(cx, None),
            height_to: create_signal(cx, None),
            height_valid,

            resolution_unit: create_signal(cx, ResolutionUnit::Inch),

            x_resolution_from: create_signal(cx, None),
            x_resolution_to: create_signal(cx, None),
            x_resolution_valid,

            y_resolution_from: create_signal(cx, None),
            y_resolution_to: create_signal(cx, None),
            y_resolution_valid,

            f_number_from: create_signal(cx, None),
            f_number_to: create_signal(cx, None),
            f_number_valid,

            focal_length_from: create_signal(cx, None),
            focal_length_to: create_signal(cx, None),
            focal_length_valid,

            exposure_time_from: create_signal(cx, None),
            exposure_time_to: create_signal(cx, None),
            exposure_time_valid,

            flash_fired: create_signal(cx, None),

            any_invalid,
        }
    }

    pub fn to_request(&self) -> ImageSearchRequest {
        ImageSearchRequest {
            image_make_enabled: *self.image_make_enabled.get(),
            image_model_enabled: *self.image_model_enabled.get(),
            image_software_enabled: *self.image_software_enabled.get(),
            width_from: *self.width_from.get(),
            width_to: *self.width_to.get(),
            height_from: *self.height_from.get(),
            height_to: *self.height_to.get(),
            resolution_unit: *self.resolution_unit.get(),
            x_resolution_from: *self.x_resolution_from.get(),
            x_resolution_to: *self.x_resolution_to.get(),
            y_resolution_from: *self.y_resolution_from.get(),
            y_resolution_to: *self.y_resolution_to.get(),
            f_number_from: *self.f_number_from.get(),
            f_number_to: *self.f_number_to.get(),
            focal_length_from: *self.focal_length_from.get(),
            focal_length_to: *self.focal_length_to.get(),
            exposure_time_from: *self.exposure_time_from.get(),
            exposure_time_to: *self.exposure_time_to.get(),
            flash_fired: *self.flash_fired.get(),
        }
    }

    pub fn update_from_request(&mut self, request: ImageSearchRequest) {
        self.image_make_enabled.set(request.image_make_enabled);
        self.image_model_enabled.set(request.image_model_enabled);
        self.image_software_enabled
            .set(request.image_software_enabled);
        self.width_from.set(request.width_from);
        self.width_to.set(request.width_to);
        self.height_from.set(request.height_from);
        self.height_to.set(request.height_to);
        self.resolution_unit.set(request.resolution_unit);
        self.x_resolution_from.set(request.x_resolution_from);
        self.x_resolution_to.set(request.x_resolution_to);
        self.y_resolution_from.set(request.y_resolution_from);
        self.y_resolution_to.set(request.y_resolution_to);
        self.f_number_from.set(request.f_number_from);
        self.f_number_to.set(request.f_number_to);
        self.focal_length_from.set(request.focal_length_from);
        self.focal_length_to.set(request.focal_length_to);
        self.exposure_time_from.set(request.exposure_time_from);
        self.exposure_time_to.set(request.exposure_time_to);
        self.flash_fired.set(request.flash_fired);
    }
}

#[component(inline_props)]
pub fn ImageFilters<'a, G: Html>(cx: Scope<'a>, data: &'a Signal<ImageFiltersData<'a>>) -> View<G> {
    const IMAGE_SIZE_MIN: u32 = 1;
    const IMAGE_SIZE_MAX: u32 = 99999;
    const RESOLUTION_MIN: f32 = 0.0;
    const RESOLUTION_MAX: f32 = 10000.0;
    const F_NUMBER_MIN: f32 = 0.0;
    const F_NUMBER_MAX: f32 = 1000.0;
    const FOCAL_LENGTH_MIN: f32 = 0.0;
    const FOCAL_LENGTH_MAX: f32 = 100.0;
    const EXPOSURE_TIME_MIN: f32 = 0.0;
    const EXPOSURE_TIME_MAX: f32 = 1000.0;

    let resolution_unit_options = create_signal(
        cx,
        vec![
            (ResolutionUnit::Inch, get_translation("inch", None)),
            (ResolutionUnit::Cm, get_translation("cm", None)),
        ],
    );

    view! { cx,
        details {
            summary { (get_translation("image_properties", None)) }

            fieldset {
                legend { (get_translation("filter_text_search", None)) }
                CheckboxFilter(text=get_translation("filter_device_manufacturer", None),
                    id="image_make", value_enabled=data.get().image_make_enabled)
                CheckboxFilter(text=get_translation("filter_device_model", None),
                    id="image_model", value_enabled=data.get().image_model_enabled)
                CheckboxFilter(text=get_translation("filter_image_software", None),
                    id="image_software", value_enabled=data.get().image_software_enabled)
            }

            NumberFilter(legend=get_translation("filter_width", None), id="width",
                min=IMAGE_SIZE_MIN, max=IMAGE_SIZE_MAX,
                value_from=data.get().width_from, value_to=data.get().width_to, valid=data.get().width_valid)

            NumberFilter(legend=get_translation("filter_height", None), id="height",
                min=IMAGE_SIZE_MIN, max=IMAGE_SIZE_MAX,
                value_from=data.get().height_from, value_to=data.get().height_to, valid=data.get().height_valid)

            NumberFilter(legend=get_translation("filter_x_resolution", None), id="x_resolution",
                min=RESOLUTION_MIN, max=RESOLUTION_MAX,
                value_from=data.get().x_resolution_from, value_to=data.get().x_resolution_to, valid=data.get().x_resolution_valid)

            NumberFilter(legend=get_translation("filter_y_resolution", None), id="y_resolution",
                min=RESOLUTION_MIN, max=RESOLUTION_MAX,
                value_from=data.get().y_resolution_from, value_to=data.get().y_resolution_to, valid=data.get().y_resolution_valid)

            NumberFilter(legend=get_translation("filter_f_number", None), id="f_number",
                min=F_NUMBER_MIN, max=F_NUMBER_MAX,
                value_from=data.get().f_number_from, value_to=data.get().f_number_to, valid=data.get().f_number_valid)

            NumberFilter(legend=get_translation("filter_focal_length", None), id="focal_length",
                min=FOCAL_LENGTH_MIN, max=FOCAL_LENGTH_MAX,
                value_from=data.get().focal_length_from, value_to=data.get().focal_length_to, valid=data.get().focal_length_valid)

            NumberFilter(legend=get_translation("filter_exposure_time", None), id="exposure_time",
                min=EXPOSURE_TIME_MIN, max=EXPOSURE_TIME_MAX,
                value_from=data.get().exposure_time_from, value_to=data.get().exposure_time_to, valid=data.get().exposure_time_valid)

            fieldset {
                legend { (get_translation("other", None)) }
                SelectFilter(text=get_translation("filter_resolution_unit", None), id="resolution_unit",
                    options=resolution_unit_options, value=data.get().resolution_unit)
                CheckboxOptionFilter(text=get_translation("filter_flash", None), id="flash_fired",
                    value_enabled=data.get().flash_fired)
            }
        }
    }
}

#[derive(Clone)]
pub struct MultimediaFiltersData<'a> {
    artist_enabled: &'a Signal<bool>,
    album_enabled: &'a Signal<bool>,
    genre_enabled: &'a Signal<bool>,
    track_number_enabled: &'a Signal<bool>,
    disc_number_enabled: &'a Signal<bool>,
    release_date_enabled: &'a Signal<bool>,

    duration_min_from: &'a Signal<Option<f32>>,
    duration_min_to: &'a Signal<Option<f32>>,
    duration_min_valid: &'a Signal<bool>,

    audio_sample_rate_from: &'a Signal<Option<u32>>,
    audio_sample_rate_to: &'a Signal<Option<u32>>,
    audio_sample_rate_valid: &'a Signal<bool>,

    audio_channel_type: &'a Signal<Option<AudioChannelType>>,

    pub any_invalid: &'a ReadSignal<bool>,
}

impl<'a> MultimediaFiltersData<'a> {
    pub fn new(cx: Scope<'a>) -> Self {
        let duration_min_valid = create_signal(cx, true);
        let audio_sample_rate_valid = create_signal(cx, true);
        let any_invalid = create_memo(cx, || {
            !*duration_min_valid.get() || !*audio_sample_rate_valid.get()
        });

        Self {
            artist_enabled: create_signal(cx, true),
            album_enabled: create_signal(cx, true),
            genre_enabled: create_signal(cx, true),
            track_number_enabled: create_signal(cx, true),
            disc_number_enabled: create_signal(cx, true),
            release_date_enabled: create_signal(cx, true),

            duration_min_from: create_signal(cx, None),
            duration_min_to: create_signal(cx, None),
            duration_min_valid,

            audio_sample_rate_from: create_signal(cx, None),
            audio_sample_rate_to: create_signal(cx, None),
            audio_sample_rate_valid,

            audio_channel_type: create_signal(cx, None),

            any_invalid,
        }
    }

    pub fn to_request(&self) -> MultimediaSearchRequest {
        MultimediaSearchRequest {
            artist_enabled: *self.artist_enabled.get(),
            album_enabled: *self.album_enabled.get(),
            genre_enabled: *self.genre_enabled.get(),
            track_number_enabled: *self.track_number_enabled.get(),
            disc_number_enabled: *self.disc_number_enabled.get(),
            release_date_enabled: *self.release_date_enabled.get(),
            duration_min_from: *self.duration_min_from.get(),
            duration_min_to: *self.duration_min_to.get(),
            audio_sample_rate_from: *self.audio_sample_rate_from.get(),
            audio_sample_rate_to: *self.audio_sample_rate_to.get(),
            audio_channel_type: *self.audio_channel_type.get(),
        }
    }

    pub fn update_from_request(&mut self, request: MultimediaSearchRequest) {
        self.artist_enabled.set(request.artist_enabled);
        self.album_enabled.set(request.album_enabled);
        self.genre_enabled.set(request.genre_enabled);
        self.track_number_enabled.set(request.track_number_enabled);
        self.disc_number_enabled.set(request.disc_number_enabled);
        self.release_date_enabled.set(request.release_date_enabled);
        self.duration_min_from.set(request.duration_min_from);
        self.duration_min_to.set(request.duration_min_to);
        self.audio_sample_rate_from
            .set(request.audio_sample_rate_from);
        self.audio_sample_rate_to.set(request.audio_sample_rate_to);
        self.audio_channel_type.set(request.audio_channel_type);
    }
}

#[component(inline_props)]
pub fn MultimediaFilters<'a, G: Html>(
    cx: Scope<'a>,
    data: &'a Signal<MultimediaFiltersData<'a>>,
) -> View<G> {
    const DURATION_MIN_MIN: f32 = 0.0;
    const DURATION_MIN_MAX: f32 = 10000.0;
    const AUDIO_SAMPLE_RATE_MIN: u32 = 0;
    const AUDIO_SAMPLE_RATE_MAX: u32 = 1000000;

    let audio_channel_type_options = create_signal(
        cx,
        vec![
            (AudioChannelType::Mono, get_translation("audio_mono", None)),
            (
                AudioChannelType::Stereo,
                get_translation("audio_stereo", None),
            ),
            (AudioChannelType::_5_1, get_translation("audio_5_1", None)),
            (AudioChannelType::_7_1, get_translation("audio_7_1", None)),
            (AudioChannelType::_16, get_translation("audio_16", None)),
            (
                AudioChannelType::Other,
                get_translation("audio_other", None),
            ),
        ],
    );

    view! { cx,
        details {
            summary { (get_translation("multimedia_properties", None)) }

            fieldset {
                legend { (get_translation("filter_text_search", None)) }
                CheckboxFilter(text=get_translation("filter_artist", None),
                    id="artist", value_enabled=data.get().artist_enabled)
                CheckboxFilter(text=get_translation("filter_album", None),
                    id="album", value_enabled=data.get().album_enabled)
                CheckboxFilter(text=get_translation("filter_genre", None),
                    id="genre", value_enabled=data.get().genre_enabled)
                CheckboxFilter(text=get_translation("filter_track_number", None),
                    id="track_number", value_enabled=data.get().track_number_enabled)
                CheckboxFilter(text=get_translation("filter_disc_number", None),
                    id="disc_number", value_enabled=data.get().disc_number_enabled)
                CheckboxFilter(text=get_translation("filter_release_date", None),
                    id="release_date", value_enabled=data.get().release_date_enabled)
            }

            NumberFilter(legend=get_translation("filter_duration_min", None), id="duration_min",
                min=DURATION_MIN_MIN, max=DURATION_MIN_MAX,
                value_from=data.get().duration_min_from, value_to=data.get().duration_min_to, valid=data.get().duration_min_valid)

            NumberFilter(legend=get_translation("filter_audio_sample_rate", None), id="audio_sample_rate",
                min=AUDIO_SAMPLE_RATE_MIN, max=AUDIO_SAMPLE_RATE_MAX,
                value_from=data.get().audio_sample_rate_from, value_to=data.get().audio_sample_rate_to, valid=data.get().audio_sample_rate_valid)

            fieldset {
                legend { (get_translation("other", None)) }
                SelectOptionFilter(text=get_translation("filter_audio_channel_type", None), id="audio_channel_type",
                    options=audio_channel_type_options, value=data.get().audio_channel_type)
            }
        }
    }
}

#[derive(Clone)]
pub struct DocumentFiltersData<'a> {
    title_enabled: &'a Signal<bool>,
    creator_enabled: &'a Signal<bool>,

    doc_created_from: &'a Signal<Option<DateTime<Utc>>>,
    doc_created_to: &'a Signal<Option<DateTime<Utc>>>,
    doc_created_valid: &'a Signal<bool>,

    doc_modified_from: &'a Signal<Option<DateTime<Utc>>>,
    doc_modified_to: &'a Signal<Option<DateTime<Utc>>>,
    doc_modified_valid: &'a Signal<bool>,

    num_pages_from: &'a Signal<Option<u32>>,
    num_pages_to: &'a Signal<Option<u32>>,
    num_pages_valid: &'a Signal<bool>,

    num_words_from: &'a Signal<Option<u32>>,
    num_words_to: &'a Signal<Option<u32>>,
    num_words_valid: &'a Signal<bool>,

    num_characters_from: &'a Signal<Option<u32>>,
    num_characters_to: &'a Signal<Option<u32>>,
    num_characters_valid: &'a Signal<bool>,

    pub any_invalid: &'a ReadSignal<bool>,
}

impl<'a> DocumentFiltersData<'a> {
    pub fn new(cx: Scope<'a>) -> Self {
        let doc_created_valid = create_signal(cx, true);
        let doc_modified_valid = create_signal(cx, true);
        let num_pages_valid = create_signal(cx, true);
        let num_words_valid = create_signal(cx, true);
        let num_characters_valid = create_signal(cx, true);
        let any_invalid = create_memo(cx, || {
            !*doc_created_valid.get()
                || !*doc_modified_valid.get()
                || !*num_pages_valid.get()
                || !*num_words_valid.get()
                || !*num_characters_valid.get()
        });

        Self {
            title_enabled: create_signal(cx, true),
            creator_enabled: create_signal(cx, true),

            doc_created_from: create_signal(cx, None),
            doc_created_to: create_signal(cx, None),
            doc_created_valid,

            doc_modified_from: create_signal(cx, None),
            doc_modified_to: create_signal(cx, None),
            doc_modified_valid,

            num_pages_from: create_signal(cx, None),
            num_pages_to: create_signal(cx, None),
            num_pages_valid,

            num_words_from: create_signal(cx, None),
            num_words_to: create_signal(cx, None),
            num_words_valid,

            num_characters_from: create_signal(cx, None),
            num_characters_to: create_signal(cx, None),
            num_characters_valid,

            any_invalid,
        }
    }

    pub fn to_request(&self) -> DocumentSearchRequest {
        DocumentSearchRequest {
            title_enabled: *self.title_enabled.get(),
            creator_enabled: *self.creator_enabled.get(),
            doc_created_from: *self.doc_created_from.get(),
            doc_created_to: *self.doc_created_to.get(),
            doc_modified_from: *self.doc_modified_from.get(),
            doc_modified_to: *self.doc_modified_to.get(),
            num_pages_from: *self.num_pages_from.get(),
            num_pages_to: *self.num_pages_to.get(),
            num_words_from: *self.num_words_from.get(),
            num_words_to: *self.num_words_to.get(),
            num_characters_from: *self.num_characters_from.get(),
            num_characters_to: *self.num_characters_to.get(),
        }
    }

    pub fn update_from_request(&mut self, request: DocumentSearchRequest) {
        self.title_enabled.set(request.title_enabled);
        self.creator_enabled.set(request.creator_enabled);
        self.doc_created_from.set(request.doc_created_from);
        self.doc_created_to.set(request.doc_created_to);
        self.doc_modified_from.set(request.doc_modified_from);
        self.doc_modified_to.set(request.doc_modified_to);
        self.num_pages_from.set(request.num_pages_from);
        self.num_pages_to.set(request.num_pages_to);
        self.num_words_from.set(request.num_words_from);
        self.num_words_to.set(request.num_words_to);
        self.num_characters_from.set(request.num_characters_from);
        self.num_characters_to.set(request.num_characters_to);
    }
}

#[component(inline_props)]
pub fn DocumentFilters<'a, G: Html>(
    cx: Scope<'a>,
    data: &'a Signal<DocumentFiltersData<'a>>,
) -> View<G> {
    view! { cx,
        details {
            summary { (get_translation("document_properties", None)) }

            fieldset {
                legend { (get_translation("filter_text_search", None)) }
                CheckboxFilter(text=get_translation("filter_title", None),
                    id="title", value_enabled=data.get().title_enabled)
                CheckboxFilter(text=get_translation("filter_creator", None),
                    id="creator", value_enabled=data.get().creator_enabled)
            }

            DateTimeFilter(legend=get_translation("filter_doc_created", None), id="doc_created",
                value_from=data.get().doc_created_from, value_to=data.get().doc_created_to,
                valid=data.get().doc_created_valid)

            DateTimeFilter(legend=get_translation("filter_doc_modified", None), id="doc_modified",
                value_from=data.get().doc_modified_from, value_to=data.get().doc_modified_to,
                valid=data.get().doc_modified_valid)

            NumberFilter(legend=get_translation("filter_num_pages", None), id="num_pages",
                min=1, max=u32::MAX,
                value_from=data.get().num_pages_from, value_to=data.get().num_pages_to,
                valid=data.get().num_pages_valid)

            NumberFilter(legend=get_translation("filter_num_words", None), id="num_words",
                min=1, max=u32::MAX,
                value_from=data.get().num_words_from, value_to=data.get().num_words_to,
                valid=data.get().num_words_valid)

            NumberFilter(legend=get_translation("filter_num_characters", None), id="num_characters",
                min=1, max=u32::MAX,
                value_from=data.get().num_characters_from, value_to=data.get().num_characters_to,
                valid=data.get().num_characters_valid)
        }
    }
}
