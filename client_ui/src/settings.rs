use std::net::SocketAddr;

use common_lib::settings::{NNServerSettings, Settings};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use url::Url;
use wasm_bindgen::JsValue;

use crate::app::{fetch, fetch_empty, widgets::StatusDialogState};

use self::widgets::{
    CheckboxSetting, DirectoryItem, DirectoryList, NumberSetting, SimpleTextSetting, TextSetting,
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
const MAX_SENTENCES_MIN: u32 = 1;
const MAX_SENTENCES_MAX: u32 = 1000;
const WINDOW_SIZE_MIN: u32 = 10;
const WINDOW_SIZE_MAX: u32 = 200;
const WINDOW_STEP_MIN: u32 = 1;
const WINDOW_STEP_MAX: u32 = 200;

trait SettingsUi {
    fn get_indexer_address_str(&self) -> String;
    fn get_elasticsearch_url_str(&self) -> String;
    fn get_tika_url_str(&self) -> String;
    fn get_nnserver_url_str(&self) -> String;
    fn get_open_on_start(&self) -> bool;
    fn get_indexing_directories_dir_items(&self) -> Vec<DirectoryItem>;
    fn get_exclude_file_regex(&self) -> String;
    fn get_watcher_enabled(&self) -> bool;
    fn get_debouncer_timeout_str(&self) -> String;
    fn get_max_file_size_str(&self) -> String;
    fn get_max_concurrent_files_str(&self) -> String;
    fn get_elasticsearch_batch_size_str(&self) -> String;
    fn get_results_per_page_str(&self) -> String;
    fn get_knn_candidates_multiplier_str(&self) -> String;
    fn get_nnserver_address_str(&self) -> String;
    fn get_text_search_enabled(&self) -> bool;
    fn get_image_search_enabled(&self) -> bool;
    fn get_reranking_enabled(&self) -> bool;
    fn get_max_sentences_str(&self) -> String;
    fn get_window_size_str(&self) -> String;
    fn get_window_step_str(&self) -> String;

    fn valid_indexer_address(indexer_address_str: &str) -> bool;
    fn valid_elasticsearch_url(elasticsearch_url_str: &str) -> bool;
    fn valid_tika_url(tika_url_str: &str) -> bool;
    fn valid_nnserver_url(nnserver_url_str: &str) -> bool;
    fn valid_debouncer_timeout(debouncer_timeout_str: &str) -> bool;
    fn valid_max_file_size(max_file_size_str: &str) -> bool;
    fn valid_max_concurrent_files(max_concurrent_files_str: &str) -> bool;
    fn valid_elasticsearch_batch_size(elasticsearch_batch_size_str: &str) -> bool;
    fn valid_results_per_page(results_per_page_str: &str) -> bool;
    fn valid_knn_candidates_multiplier(knn_candidates_multiplier_str: &str) -> bool;
    fn valid_nnserver_address(nnserver_address_str: &str) -> bool;
    fn valid_max_sentences(max_sentences_str: &str) -> bool;
    fn valid_window_size(window_size_str: &str) -> bool;
    fn valid_window_step(window_step_str: &str) -> bool;

    #[allow(clippy::too_many_arguments)]
    fn parse(
        indexer_address_str: &str,
        elasticsearch_url_str: &str,
        tika_url_str: &str,
        nnserver_url_str: &str,
        open_on_start: bool,
        indexing_directories_dir_items: &[DirectoryItem],
        exclude_file_regex: &str,
        watcher_enabled: bool,
        debouncer_timeout_str: &str,
        max_file_size_str: &str,
        max_concurrent_files_str: &str,
        elasticsearch_batch_size_str: &str,
        results_per_page_str: &str,
        knn_candidates_multiplier_str: &str,
        nnserver_address_str: &str,
        text_search_enabled: bool,
        image_search_enabled: bool,
        reranking_enabled: bool,
        max_sentences_str: &str,
        window_size_str: &str,
        window_step_str: &str,
    ) -> Self;
}

impl SettingsUi for Settings {
    fn get_indexer_address_str(&self) -> String {
        self.indexer_address.to_string()
    }
    fn get_elasticsearch_url_str(&self) -> String {
        self.elasticsearch_url.to_string()
    }
    fn get_tika_url_str(&self) -> String {
        self.tika_url.to_string()
    }
    fn get_nnserver_url_str(&self) -> String {
        self.nnserver_url.to_string()
    }
    fn get_open_on_start(&self) -> bool {
        self.open_on_start
    }
    fn get_indexing_directories_dir_items(&self) -> Vec<DirectoryItem> {
        self.indexing_directories
            .iter()
            .map(|p| DirectoryItem::new(p.clone()))
            .collect()
    }
    fn get_exclude_file_regex(&self) -> String {
        self.exclude_file_regex.clone()
    }
    fn get_watcher_enabled(&self) -> bool {
        self.watcher_enabled
    }
    fn get_debouncer_timeout_str(&self) -> String {
        self.debouncer_timeout.to_string()
    }
    fn get_max_file_size_str(&self) -> String {
        ((self.max_file_size as f64) / 1024.0 / 1024.0).to_string()
    }
    fn get_max_concurrent_files_str(&self) -> String {
        self.max_concurrent_files.to_string()
    }
    fn get_elasticsearch_batch_size_str(&self) -> String {
        self.elasticsearch_batch_size.to_string()
    }
    fn get_results_per_page_str(&self) -> String {
        self.results_per_page.to_string()
    }
    fn get_knn_candidates_multiplier_str(&self) -> String {
        self.knn_candidates_multiplier.to_string()
    }
    fn get_nnserver_address_str(&self) -> String {
        self.nn_server.nnserver_address.to_string()
    }
    fn get_text_search_enabled(&self) -> bool {
        self.nn_server.text_search_enabled
    }
    fn get_image_search_enabled(&self) -> bool {
        self.nn_server.image_search_enabled
    }
    fn get_reranking_enabled(&self) -> bool {
        self.nn_server.reranking_enabled
    }
    fn get_max_sentences_str(&self) -> String {
        self.nn_server.max_sentences.to_string()
    }
    fn get_window_size_str(&self) -> String {
        self.nn_server.window_size.to_string()
    }
    fn get_window_step_str(&self) -> String {
        self.nn_server.window_step.to_string()
    }

    fn valid_indexer_address(indexer_address_str: &str) -> bool {
        indexer_address_str.parse::<SocketAddr>().is_ok()
    }
    fn valid_elasticsearch_url(elasticsearch_url_str: &str) -> bool {
        Url::parse(elasticsearch_url_str).is_ok()
    }
    fn valid_tika_url(tika_url_str: &str) -> bool {
        Url::parse(tika_url_str).is_ok()
    }
    fn valid_nnserver_url(nnserver_url_str: &str) -> bool {
        Url::parse(nnserver_url_str).is_ok()
    }
    fn valid_debouncer_timeout(debouncer_timeout_str: &str) -> bool {
        debouncer_timeout_str
            .parse()
            .map(|x: f32| (DEBOUNCER_TIMEOUT_MIN..=DEBOUNCER_TIMEOUT_MAX).contains(&x))
            == Ok(true)
    }
    fn valid_max_file_size(max_file_size_str: &str) -> bool {
        max_file_size_str
            .parse()
            .map(|x: f64| (MAX_FILE_SIZE_MIN..=MAX_FILE_SIZE_MAX).contains(&x))
            == Ok(true)
    }
    fn valid_max_concurrent_files(max_concurrent_files_str: &str) -> bool {
        max_concurrent_files_str
            .parse()
            .map(|x: usize| (MAX_CONCURRENT_FILES_MIN..=MAX_CONCURRENT_FILES_MAX).contains(&x))
            == Ok(true)
    }
    fn valid_elasticsearch_batch_size(elasticsearch_batch_size_str: &str) -> bool {
        elasticsearch_batch_size_str.parse().map(|x: usize| {
            (ELASTICSEARCH_BATCH_SIZE_MIN..=ELASTICSEARCH_BATCH_SIZE_MAX).contains(&x)
        }) == Ok(true)
    }
    fn valid_results_per_page(results_per_page_str: &str) -> bool {
        results_per_page_str
            .parse()
            .map(|x: u32| (RESULTS_PER_PAGE_MIN..=RESULTS_PER_PAGE_MAX).contains(&x))
            == Ok(true)
    }
    fn valid_knn_candidates_multiplier(knn_candidates_multiplier_str: &str) -> bool {
        knn_candidates_multiplier_str.parse().map(|x: u32| {
            (KNN_CANDIDATES_MULTIPLIER_MIN..=KNN_CANDIDATES_MULTIPLIER_MAX).contains(&x)
        }) == Ok(true)
    }
    fn valid_nnserver_address(nnserver_address_str: &str) -> bool {
        nnserver_address_str.parse::<SocketAddr>().is_ok()
    }
    fn valid_max_sentences(max_sentences_str: &str) -> bool {
        max_sentences_str
            .parse()
            .map(|x: u32| (MAX_SENTENCES_MIN..=MAX_SENTENCES_MAX).contains(&x))
            == Ok(true)
    }
    fn valid_window_size(window_size_str: &str) -> bool {
        window_size_str
            .parse()
            .map(|x: u32| (WINDOW_SIZE_MIN..=WINDOW_SIZE_MAX).contains(&x))
            == Ok(true)
    }
    fn valid_window_step(window_step_str: &str) -> bool {
        window_step_str
            .parse()
            .map(|x: u32| (WINDOW_STEP_MIN..=WINDOW_STEP_MAX).contains(&x))
            == Ok(true)
    }

    fn parse(
        indexer_address_str: &str,
        elasticsearch_url_str: &str,
        tika_url_str: &str,
        nnserver_url_str: &str,
        open_on_start: bool,
        indexing_directories_dir_items: &[DirectoryItem],
        exclude_file_regex: &str,
        watcher_enabled: bool,
        debouncer_timeout_str: &str,
        max_file_size_str: &str,
        max_concurrent_files_str: &str,
        elasticsearch_batch_size_str: &str,
        results_per_page_str: &str,
        knn_candidates_multiplier_str: &str,
        nnserver_address_str: &str,
        text_search_enabled: bool,
        image_search_enabled: bool,
        reranking_enabled: bool,
        max_sentences_str: &str,
        window_size_str: &str,
        window_step_str: &str,
    ) -> Self {
        Self {
            indexer_address: indexer_address_str.parse().unwrap(),
            elasticsearch_url: Url::parse(elasticsearch_url_str).unwrap(),
            tika_url: Url::parse(tika_url_str).unwrap(),
            nnserver_url: Url::parse(nnserver_url_str).unwrap(),
            open_on_start,
            indexing_directories: indexing_directories_dir_items
                .iter()
                .map(|f| f.dir.clone())
                .collect(),
            exclude_file_regex: exclude_file_regex.to_owned(),
            watcher_enabled,
            debouncer_timeout: debouncer_timeout_str.parse().unwrap(),
            max_file_size: (max_file_size_str.parse::<f64>().unwrap() * 1024.0 * 1024.0) as u64,
            max_concurrent_files: max_concurrent_files_str.parse().unwrap(),
            elasticsearch_batch_size: elasticsearch_batch_size_str.parse().unwrap(),
            results_per_page: results_per_page_str.parse().unwrap(),
            knn_candidates_multiplier: knn_candidates_multiplier_str.parse().unwrap(),
            nn_server: NNServerSettings {
                nnserver_address: nnserver_address_str.parse().unwrap(),
                text_search_enabled,
                image_search_enabled,
                reranking_enabled,
                max_sentences: max_sentences_str.parse().unwrap(),
                window_size: window_size_str.parse().unwrap(),
                window_step: window_step_str.parse().unwrap(),
            },
        }
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
    let indexer_address_str = create_signal(cx, settings.get().get_indexer_address_str());
    let elasticsearch_url_str = create_signal(cx, settings.get().get_elasticsearch_url_str());
    let tika_url_str = create_signal(cx, settings.get().get_tika_url_str());
    let nnserver_url_str = create_signal(cx, settings.get().get_nnserver_url_str());
    let open_on_start = create_signal(cx, settings.get().get_open_on_start());
    let indexing_directories =
        create_signal(cx, settings.get().get_indexing_directories_dir_items());
    let exclude_file_regex = create_signal(cx, settings.get().get_exclude_file_regex());
    let watcher_enabled = create_signal(cx, settings.get().get_watcher_enabled());
    let debouncer_timeout_str = create_signal(cx, settings.get().get_debouncer_timeout_str());
    let max_file_size_str = create_signal(cx, settings.get().get_max_file_size_str());
    let max_concurrent_files_str = create_signal(cx, settings.get().get_max_concurrent_files_str());
    let elasticsearch_batch_size_str =
        create_signal(cx, settings.get().get_elasticsearch_batch_size_str());
    let results_per_page_str = create_signal(cx, settings.get().get_results_per_page_str());
    let knn_candidates_multiplier_str =
        create_signal(cx, settings.get().get_knn_candidates_multiplier_str());
    let nnserver_address_str = create_signal(cx, settings.get().get_nnserver_address_str());
    let text_search_enabled = create_signal(cx, settings.get().get_text_search_enabled());
    let image_search_enabled = create_signal(cx, settings.get().get_image_search_enabled());
    let reranking_enabled = create_signal(cx, settings.get().get_reranking_enabled());
    let max_sentences_str = create_signal(cx, settings.get().get_max_sentences_str());
    let window_size_str = create_signal(cx, settings.get().get_window_size_str());
    let window_step_str = create_signal(cx, settings.get().get_window_step_str());

    // Validation values for settings
    let indexer_address_valid = create_memo(cx, || {
        Settings::valid_indexer_address(&indexer_address_str.get())
    });
    let elasticsearch_url_valid = create_memo(cx, || {
        Settings::valid_elasticsearch_url(&elasticsearch_url_str.get())
    });
    let tika_url_valid = create_memo(cx, || Settings::valid_tika_url(&tika_url_str.get()));
    let nnserver_url_valid =
        create_memo(cx, || Settings::valid_nnserver_url(&nnserver_url_str.get()));
    let debouncer_timeout_valid = create_memo(cx, || {
        Settings::valid_debouncer_timeout(&debouncer_timeout_str.get())
    });
    let max_file_size_valid = create_memo(cx, || {
        Settings::valid_max_file_size(&max_file_size_str.get())
    });
    let max_concurrent_files_valid = create_memo(cx, || {
        Settings::valid_max_concurrent_files(&max_concurrent_files_str.get())
    });
    let elasticsearch_batch_size_valid = create_memo(cx, || {
        Settings::valid_elasticsearch_batch_size(&elasticsearch_batch_size_str.get())
    });
    let results_per_page_valid = create_memo(cx, || {
        Settings::valid_results_per_page(&results_per_page_str.get())
    });
    let knn_candidates_multiplier_valid = create_memo(cx, || {
        Settings::valid_knn_candidates_multiplier(&knn_candidates_multiplier_str.get())
    });
    let nnserver_address_valid = create_memo(cx, || {
        Settings::valid_nnserver_address(&nnserver_address_str.get())
    });
    let max_sentences_valid = create_memo(cx, || {
        Settings::valid_max_sentences(&max_sentences_str.get())
    });
    let window_size_valid = create_memo(cx, || Settings::valid_window_size(&window_size_str.get()));
    let window_step_valid = create_memo(cx, || Settings::valid_window_step(&window_step_str.get()));
    let any_invalid = create_memo(cx, || {
        !*indexer_address_valid.get()
            || !*elasticsearch_url_valid.get()
            || !*tika_url_valid.get()
            || !*nnserver_url_valid.get()
            || !*debouncer_timeout_valid.get()
            || !*max_file_size_valid.get()
            || !*max_concurrent_files_valid.get()
            || !*elasticsearch_batch_size_valid.get()
            || !*results_per_page_valid.get()
            || !*knn_candidates_multiplier_valid.get()
            || !*nnserver_address_valid.get()
            || !*max_sentences_valid.get()
            || !*window_size_valid.get()
            || !*window_step_valid.get()
    });

    // Set input values from settings when they are updated (on load from server or reset)
    create_effect(cx, || {
        indexer_address_str.set(settings.get().get_indexer_address_str());
        elasticsearch_url_str.set(settings.get().get_elasticsearch_url_str());
        tika_url_str.set(settings.get().get_tika_url_str());
        nnserver_url_str.set(settings.get().get_nnserver_url_str());
        open_on_start.set(settings.get().get_open_on_start());
        indexing_directories.set(settings.get().get_indexing_directories_dir_items());
        exclude_file_regex.set(settings.get().get_exclude_file_regex());
        watcher_enabled.set(settings.get().get_watcher_enabled());
        debouncer_timeout_str.set(settings.get().get_debouncer_timeout_str());
        max_file_size_str.set(settings.get().get_max_file_size_str());
        max_concurrent_files_str.set(settings.get().get_max_concurrent_files_str());
        elasticsearch_batch_size_str.set(settings.get().get_elasticsearch_batch_size_str());
        results_per_page_str.set(settings.get().get_results_per_page_str());
        knn_candidates_multiplier_str.set(settings.get().get_knn_candidates_multiplier_str());
        nnserver_address_str.set(settings.get().get_nnserver_address_str());
        text_search_enabled.set(settings.get().get_text_search_enabled());
        image_search_enabled.set(settings.get().get_image_search_enabled());
        reranking_enabled.set(settings.get().get_reranking_enabled());
        max_sentences_str.set(settings.get().get_max_sentences_str());
        window_size_str.set(settings.get().get_window_size_str());
        window_step_str.set(settings.get().get_window_step_str());
    });
    let reset_settings = |_| {
        settings.trigger_subscribers();
    };

    // Load settings
    spawn_local_scoped(cx, async move {
        status_dialog_state.set(StatusDialogState::Loading);

        match get_settings().await {
            Ok(res) => {
                settings.set(res);
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

            let new_settings = Settings::parse(
                &indexer_address_str.get(),
                &elasticsearch_url_str.get(),
                &tika_url_str.get(),
                &nnserver_url_str.get(),
                *open_on_start.get(),
                &indexing_directories.get(),
                &exclude_file_regex.get(),
                *watcher_enabled.get(),
                &debouncer_timeout_str.get(),
                &max_file_size_str.get(),
                &max_concurrent_files_str.get(),
                &elasticsearch_batch_size_str.get(),
                &results_per_page_str.get(),
                &knn_candidates_multiplier_str.get(),
                &nnserver_address_str.get(),
                *text_search_enabled.get(),
                *image_search_enabled.get(),
                *reranking_enabled.get(),
                &max_sentences_str.get(),
                &window_size_str.get(),
                &window_step_str.get(),
            );

            if let Err(e) = put_settings(&new_settings).await {
                status_dialog_state.set(StatusDialogState::Error(format!(
                    "❌ Ошибка сохранения настроек: {e:#?}",
                )));
                return;
            }

            settings.set(new_settings);
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
                            value=indexer_address_str, valid=indexer_address_valid)
                        TextSetting(id="elasticsearch_url", label="URL сервера Elasticsearch: ",
                            value=elasticsearch_url_str, valid=elasticsearch_url_valid)
                        TextSetting(id="tika_url", label="URL сервера Apache Tika: ",
                            value=tika_url_str, valid=tika_url_valid)
                        TextSetting(id="nnserver_url", label="URL сервера нейронных сетей: ",
                            value=nnserver_url_str, valid=nnserver_url_valid)
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
                        NumberSetting(id="debouncer_timeout", label="Время задержки событий файловой системы (с): ",
                            value=debouncer_timeout_str, valid=debouncer_timeout_valid)
                        NumberSetting(id="max_file_size", label="Максимальный размер файла (МиБ): ",
                            value=max_file_size_str, valid=max_file_size_valid)
                        NumberSetting(id="max_concurrent_files", label="Максимальное количество одновременно обрабатываемых документов: ",
                            value=max_concurrent_files_str, valid=max_concurrent_files_valid)
                        NumberSetting(id="elasticsearch_batch_size", label="Количество отправляемых в Elasticsearch изменений за раз: ",
                            value=elasticsearch_batch_size_str, valid=elasticsearch_batch_size_valid)
                    }

                    fieldset {
                        legend { "Настройки поиска" }
                        NumberSetting(id="results_per_page", label="Количество результатов на странице: ",
                            value=results_per_page_str, valid=results_per_page_valid)
                        NumberSetting(id="knn_candidates_multiplier", label="Множитель количества кандидатов kNN при семантическом поиске: ",
                            value=knn_candidates_multiplier_str, valid=knn_candidates_multiplier_valid)
                    }

                    fieldset {
                        legend { "Настройки сервера нейронных сетей" }
                        TextSetting(id="nnserver_address", label="Адрес сервера нейронных сетей: ",
                            value=nnserver_address_str, valid=nnserver_address_valid)
                        CheckboxSetting(id="text_search_enabled", label="Семантический поиск по тексту: ",
                            value=text_search_enabled)
                        CheckboxSetting(id="image_search_enabled", label="Семантический поиск по изображениям: ",
                            value=image_search_enabled)
                        CheckboxSetting(id="reranking_enabled", label="Переранжирование: ",
                            value=reranking_enabled)
                        NumberSetting(id="max_sentences", label="Максимальное количество обрабатываемых предложений: ",
                            value=max_sentences_str, valid=max_sentences_valid)
                        NumberSetting(id="window_size", label="Размер окна слов: ",
                            value=window_size_str, valid=window_size_valid)
                        NumberSetting(id="window_step", label="Шаг окна слов: ",
                            value=window_step_str, valid=window_step_valid)
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
