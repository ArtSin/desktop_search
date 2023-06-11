use std::{net::SocketAddr, str::FromStr};

use common_lib::settings::{NNServerSettings, Settings};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use url::Url;
use wasm_bindgen::JsValue;

use crate::app::{fetch, fetch_empty, widgets::StatusDialogState};

use self::widgets::{
    CheckboxSetting, DirectoryItem, DirectoryList, NNSetting, NNSettingsData, NumberSetting,
    SimpleTextSetting, TextSetting,
};

mod widgets;

const DEBOUNCER_TIMEOUT_MIN: f32 = 0.1;
const DEBOUNCER_TIMEOUT_MAX: f32 = 3600.0;
pub const MAX_FILE_SIZE_MIN: f64 = 0.01;
pub const MAX_FILE_SIZE_MAX: f64 = 1000.0;
const MAX_CONCURRENT_FILES_MIN: usize = 1;
const MAX_CONCURRENT_FILES_MAX: usize = 256;
const ELASTICSEARCH_BATCH_SIZE_MIN: usize = 1;
const ELASTICSEARCH_BATCH_SIZE_MAX: usize = 1000;
const RESULTS_PER_PAGE_MIN: u32 = 1;
const RESULTS_PER_PAGE_MAX: u32 = 1000;
const KNN_CANDIDATES_MULTIPLIER_MIN: u32 = 1;
const KNN_CANDIDATES_MULTIPLIER_MAX: u32 = 100;
const BATCH_SIZE_MIN: usize = 1;
const BATCH_SIZE_MAX: usize = 256;
const MAX_DELAY_MS_MIN: u64 = 10;
const MAX_DELAY_MS_MAX: u64 = 1000;
const MAX_SENTENCES_MIN: u32 = 1;
const MAX_SENTENCES_MAX: u32 = 1000;
const WINDOW_SIZE_MIN: u32 = 10;
const WINDOW_SIZE_MAX: u32 = 200;
const WINDOW_STEP_MIN: u32 = 1;
const WINDOW_STEP_MAX: u32 = 200;
const SUMMARY_LEN_MIN: u32 = 1;
const SUMMARY_LEN_MAX: u32 = 10;

trait SettingsUi {
    fn get_indexing_directories_dir_items(&self) -> Vec<DirectoryItem>;
    fn get_max_file_size_mib(&self) -> f64;
}

impl SettingsUi for Settings {
    fn get_indexing_directories_dir_items(&self) -> Vec<DirectoryItem> {
        self.indexing_directories
            .iter()
            .map(|p| DirectoryItem::new(p.clone()))
            .collect()
    }
    fn get_max_file_size_mib(&self) -> f64 {
        (self.max_file_size as f64) / 1024.0 / 1024.0
    }
}

async fn get_settings() -> Result<Settings, JsValue> {
    fetch("/settings", "GET", None::<&()>).await
}

async fn put_settings(settings: &Settings) -> Result<(), JsValue> {
    fetch_empty("/settings", "PUT", Some(settings)).await
}

#[component(inline_props)]
pub fn Settings<'a, G: Html>(
    cx: Scope<'a>,
    settings: &'a Signal<Settings>,
    status_dialog_state: &'a Signal<StatusDialogState>,
) -> View<G> {
    // Input values for settings
    let indexer_address = create_signal(cx, settings.get().indexer_address);
    let elasticsearch_url = create_signal(cx, settings.get().elasticsearch_url.clone());
    let tika_url = create_signal(cx, settings.get().tika_url.clone());
    let nn_server_url = create_signal(cx, settings.get().nn_server_url.clone());
    let open_on_start = create_signal(cx, settings.get().open_on_start);
    let indexing_directories =
        create_signal(cx, settings.get().get_indexing_directories_dir_items());
    let exclude_file_regex = create_signal(cx, settings.get().exclude_file_regex.clone());
    let watcher_enabled = create_signal(cx, settings.get().watcher_enabled);
    let debouncer_timeout = create_signal(cx, settings.get().debouncer_timeout);
    let max_file_size = create_signal(cx, settings.get().get_max_file_size_mib());
    let max_concurrent_files = create_signal(cx, settings.get().max_concurrent_files);
    let elasticsearch_batch_size = create_signal(cx, settings.get().elasticsearch_batch_size);
    let results_per_page = create_signal(cx, settings.get().results_per_page);
    let knn_candidates_multiplier = create_signal(cx, settings.get().knn_candidates_multiplier);
    let nn_server_address = create_signal(cx, settings.get().nn_server.nn_server_address);
    let text_search_enabled = create_signal(cx, settings.get().nn_server.text_search_enabled);
    let image_search_enabled = create_signal(cx, settings.get().nn_server.image_search_enabled);
    let reranking_enabled = create_signal(cx, settings.get().nn_server.reranking_enabled);
    let clip_image_data = create_signal(
        cx,
        NNSettingsData::new(cx, &settings.get().nn_server.clip_image),
    );
    let clip_text_data = create_signal(
        cx,
        NNSettingsData::new(cx, &settings.get().nn_server.clip_text),
    );
    let minilm_text_data = create_signal(
        cx,
        NNSettingsData::new(cx, &settings.get().nn_server.minilm_text),
    );
    let minilm_rerank_data = create_signal(
        cx,
        NNSettingsData::new(cx, &settings.get().nn_server.minilm_rerank),
    );
    let max_sentences = create_signal(cx, settings.get().nn_server.max_sentences);
    let window_size = create_signal(cx, settings.get().nn_server.window_size);
    let window_step = create_signal(cx, settings.get().nn_server.window_step);
    let summary_len = create_signal(cx, settings.get().nn_server.summary_len);

    // Validation values for settings
    let indexer_address_valid = create_signal(cx, true);
    let elasticsearch_url_valid = create_signal(cx, true);
    let tika_url_valid = create_signal(cx, true);
    let nn_server_url_valid = create_signal(cx, true);
    let debouncer_timeout_valid = create_signal(cx, true);
    let max_file_size_valid = create_signal(cx, true);
    let max_concurrent_files_valid = create_signal(cx, true);
    let elasticsearch_batch_size_valid = create_signal(cx, true);
    let results_per_page_valid = create_signal(cx, true);
    let knn_candidates_multiplier_valid = create_signal(cx, true);
    let nn_server_address_valid = create_signal(cx, true);
    let max_sentences_valid = create_signal(cx, true);
    let window_size_valid = create_signal(cx, true);
    let window_step_valid = create_signal(cx, true);
    let summary_len_valid = create_signal(cx, true);
    let any_invalid = create_memo(cx, || {
        !*indexer_address_valid.get()
            || !*elasticsearch_url_valid.get()
            || !*tika_url_valid.get()
            || !*nn_server_url_valid.get()
            || !*debouncer_timeout_valid.get()
            || !*max_file_size_valid.get()
            || !*max_concurrent_files_valid.get()
            || !*elasticsearch_batch_size_valid.get()
            || !*results_per_page_valid.get()
            || !*knn_candidates_multiplier_valid.get()
            || !*nn_server_address_valid.get()
            || *clip_image_data.get().any_invalid.get()
            || *clip_text_data.get().any_invalid.get()
            || *minilm_text_data.get().any_invalid.get()
            || *minilm_rerank_data.get().any_invalid.get()
            || !*max_sentences_valid.get()
            || !*window_size_valid.get()
            || !*window_step_valid.get()
            || !*summary_len_valid.get()
    });

    // Set input values from settings when they are updated (on load from server or reset)
    let update_settings = || {
        indexer_address.set(settings.get().indexer_address);
        elasticsearch_url.set(settings.get().elasticsearch_url.clone());
        tika_url.set(settings.get().tika_url.clone());
        nn_server_url.set(settings.get().nn_server_url.clone());
        open_on_start.set(settings.get().open_on_start);
        indexing_directories.set(settings.get().get_indexing_directories_dir_items());
        exclude_file_regex.set(settings.get().exclude_file_regex.clone());
        watcher_enabled.set(settings.get().watcher_enabled);
        debouncer_timeout.set(settings.get().debouncer_timeout);
        max_file_size.set(settings.get().get_max_file_size_mib());
        max_concurrent_files.set(settings.get().max_concurrent_files);
        elasticsearch_batch_size.set(settings.get().elasticsearch_batch_size);
        results_per_page.set(settings.get().results_per_page);
        knn_candidates_multiplier.set(settings.get().knn_candidates_multiplier);
        nn_server_address.set(settings.get().nn_server.nn_server_address);
        text_search_enabled.set(settings.get().nn_server.text_search_enabled);
        image_search_enabled.set(settings.get().nn_server.image_search_enabled);
        reranking_enabled.set(settings.get().nn_server.reranking_enabled);
        clip_image_data
            .modify()
            .update_from_settings(settings.get().nn_server.clip_image.clone());
        clip_text_data
            .modify()
            .update_from_settings(settings.get().nn_server.clip_text.clone());
        minilm_text_data
            .modify()
            .update_from_settings(settings.get().nn_server.minilm_text.clone());
        minilm_rerank_data
            .modify()
            .update_from_settings(settings.get().nn_server.minilm_rerank.clone());
        max_sentences.set(settings.get().nn_server.max_sentences);
        window_size.set(settings.get().nn_server.window_size);
        window_step.set(settings.get().nn_server.window_step);
        summary_len.set(settings.get().nn_server.summary_len);
    };
    let reset_settings = move |_| update_settings();

    // Load settings
    spawn_local_scoped(cx, async move {
        status_dialog_state.set(StatusDialogState::Loading);

        match get_settings().await {
            Ok(res) => {
                settings.set(res);
                update_settings();
                status_dialog_state.set(StatusDialogState::None);
            }
            Err(e) => {
                status_dialog_state.set(StatusDialogState::Error(format!(
                    "❌ Ошибка загрузки настроек: {e:#?}",
                )));
            }
        }
    });

    // Save settings
    let set_settings = move |_| {
        spawn_local_scoped(cx, async move {
            status_dialog_state.set(StatusDialogState::Loading);

            let new_settings = Settings {
                indexer_address: *indexer_address.get(),
                elasticsearch_url: (*elasticsearch_url.get()).clone(),
                tika_url: (*tika_url.get()).clone(),
                nn_server_url: (*nn_server_url.get()).clone(),
                open_on_start: *open_on_start.get(),
                indexing_directories: indexing_directories
                    .get()
                    .iter()
                    .map(|f| f.dir.clone())
                    .collect(),
                exclude_file_regex: (*exclude_file_regex.get()).clone(),
                watcher_enabled: *watcher_enabled.get(),
                debouncer_timeout: *debouncer_timeout.get(),
                max_file_size: (*max_file_size.get() * 1024.0 * 1024.0) as u64,
                max_concurrent_files: *max_concurrent_files.get(),
                elasticsearch_batch_size: *elasticsearch_batch_size.get(),
                results_per_page: *results_per_page.get(),
                knn_candidates_multiplier: *knn_candidates_multiplier.get(),
                nn_server: NNServerSettings {
                    nn_server_address: *nn_server_address.get(),
                    text_search_enabled: *text_search_enabled.get(),
                    image_search_enabled: *image_search_enabled.get(),
                    reranking_enabled: *reranking_enabled.get(),
                    clip_image: clip_image_data.get().to_settings(),
                    clip_text: clip_text_data.get().to_settings(),
                    minilm_text: minilm_text_data.get().to_settings(),
                    minilm_rerank: minilm_rerank_data.get().to_settings(),
                    max_sentences: *max_sentences.get(),
                    window_size: *window_size.get(),
                    window_step: *window_step.get(),
                    summary_len: *summary_len.get(),
                },
            };

            if let Err(e) = put_settings(&new_settings).await {
                status_dialog_state.set(StatusDialogState::Error(format!(
                    "❌ Ошибка сохранения настроек: {e:#?}",
                )));
                return;
            }

            settings.set(new_settings);
            update_settings();
            status_dialog_state.set(StatusDialogState::Info("✅ Настройки сохранены".to_owned()));
        })
    };

    view! { cx,
        div(class="main_container") {
            main {
                form(id="settings", on:submit=set_settings, action="javascript:void(0);") {
                    fieldset {
                        legend { "Серверные настройки" }
                        TextSetting(id="indexer_address", label="Адрес сервера индексации: ",
                            parse=SocketAddr::from_str,
                            value=indexer_address, valid=indexer_address_valid)
                        TextSetting(id="elasticsearch_url", label="URL сервера Elasticsearch: ",
                            parse=Url::parse,
                            value=elasticsearch_url, valid=elasticsearch_url_valid)
                        TextSetting(id="tika_url", label="URL сервера Apache Tika: ",
                            parse=Url::parse,
                            value=tika_url, valid=tika_url_valid)
                        TextSetting(id="nn_server_url", label="URL сервера нейронных сетей: ",
                            parse=Url::parse,
                            value=nn_server_url, valid=nn_server_url_valid)
                        CheckboxSetting(id="open_on_start", label="Открывать интерфейс при запуске сервера: ",
                            value=open_on_start)
                    }

                    fieldset {
                        legend { "Индексируемые папки" }
                        DirectoryList(directory_list=indexing_directories,
                            status_dialog_state=status_dialog_state)
                        SimpleTextSetting(id="exclude_file_regex",
                            label="Регулярное выражение для исключения файлов: ", value=exclude_file_regex)
                    }

                    fieldset {
                        legend { "Настройки индексации" }
                        CheckboxSetting(id="watcher_enabled", label="Отслеживать изменения файлов: ",
                            value=watcher_enabled)
                        NumberSetting(id="debouncer_timeout".to_owned(),
                            label="Время задержки событий файловой системы (с): ".to_owned(),
                            min=DEBOUNCER_TIMEOUT_MIN, max=DEBOUNCER_TIMEOUT_MAX,
                            value=debouncer_timeout, valid=debouncer_timeout_valid)
                        NumberSetting(id="max_file_size".to_owned(),
                            label="Максимальный размер файла (МиБ): ".to_owned(),
                            min=MAX_FILE_SIZE_MIN, max=MAX_FILE_SIZE_MAX,
                            value=max_file_size, valid=max_file_size_valid)
                        NumberSetting(id="max_concurrent_files".to_owned(),
                            label="Максимальное количество одновременно обрабатываемых документов: ".to_owned(),
                            min=MAX_CONCURRENT_FILES_MIN, max=MAX_CONCURRENT_FILES_MAX,
                            value=max_concurrent_files, valid=max_concurrent_files_valid)
                        NumberSetting(id="elasticsearch_batch_size".to_owned(),
                            label="Количество отправляемых в Elasticsearch изменений за раз: ".to_owned(),
                            min=ELASTICSEARCH_BATCH_SIZE_MIN, max=ELASTICSEARCH_BATCH_SIZE_MAX,
                            value=elasticsearch_batch_size, valid=elasticsearch_batch_size_valid)
                    }

                    fieldset {
                        legend { "Настройки поиска" }
                        NumberSetting(id="results_per_page".to_owned(),
                            label="Количество результатов на странице: ".to_owned(),
                            min=RESULTS_PER_PAGE_MIN, max=RESULTS_PER_PAGE_MAX,
                            value=results_per_page, valid=results_per_page_valid)
                        NumberSetting(id="knn_candidates_multiplier".to_owned(),
                            label="Множитель количества кандидатов kNN при семантическом поиске: ".to_owned(),
                            min=KNN_CANDIDATES_MULTIPLIER_MIN, max=KNN_CANDIDATES_MULTIPLIER_MAX,
                            value=knn_candidates_multiplier, valid=knn_candidates_multiplier_valid)
                    }

                    fieldset {
                        legend { "Настройки сервера нейронных сетей" }
                        TextSetting(id="nn_server_address", label="Адрес сервера нейронных сетей: ",
                            parse=SocketAddr::from_str,
                            value=nn_server_address, valid=nn_server_address_valid)
                        CheckboxSetting(id="text_search_enabled", label="Семантический поиск по тексту: ",
                            value=text_search_enabled)
                        CheckboxSetting(id="image_search_enabled", label="Семантический поиск по изображениям: ",
                            value=image_search_enabled)
                        CheckboxSetting(id="reranking_enabled", label="Переранжирование: ",
                            value=reranking_enabled)
                        NNSetting(id="clip_image", label="CLIP (изображения)", data=clip_image_data)
                        NNSetting(id="clip_text", label="CLIP (текст)", data=clip_text_data)
                        NNSetting(id="minilm_text", label="MiniLM (текст)", data=minilm_text_data)
                        NNSetting(id="minilm_rerank", label="MiniLM (переранжирование)", data=minilm_rerank_data)
                        NumberSetting(id="max_sentences".to_owned(),
                            label="Максимальное количество обрабатываемых предложений: ".to_owned(),
                            min=MAX_SENTENCES_MIN, max=MAX_SENTENCES_MAX,
                            value=max_sentences, valid=max_sentences_valid)
                        NumberSetting(id="window_size".to_owned(),
                            label="Размер окна слов: ".to_owned(),
                            min=WINDOW_SIZE_MIN, max=WINDOW_SIZE_MAX,
                            value=window_size, valid=window_size_valid)
                        NumberSetting(id="window_step".to_owned(),
                            label="Шаг окна слов: ".to_owned(),
                            min=WINDOW_STEP_MIN, max=WINDOW_STEP_MAX,
                            value=window_step, valid=window_step_valid)
                        NumberSetting(id="summary_len".to_owned(),
                            label="Количество предложений на документ для переранжирования: ".to_owned(),
                            min=SUMMARY_LEN_MIN, max=SUMMARY_LEN_MAX,
                            value=summary_len, valid=summary_len_valid)
                    }

                    fieldset {
                        legend { "Предупреждение" }
                        p {
                            "Для применения серверных настроек и настроек сервера нейронных сетей требуется перезапуск.
                            После добавления или удаления индексируемых папок нужно выполнить индексацию.
                            Для применения настроек сервера нейронных сетей, влияющих на индексацию, таких как
                            использование семантического поиска по тексту или изображениям (и их параметров),
                            нужно выполнить переиндексацию (очистить индекс и заново проиндексировать)."
                        }
                    }

                    div(class="settings_buttons") {
                        button(type="button", on:click=reset_settings) { "Отмена" }
                        button(type="submit", disabled=*any_invalid.get()) { "Сохранить" }
                    }
                }
            }
        }
    }
}
