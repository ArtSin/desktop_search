use common_lib::settings::Settings;
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
const NNSERVER_BATCH_SIZE_MIN: usize = 1;
const NNSERVER_BATCH_SIZE_MAX: usize = 256;
const ELASTICSEARCH_BATCH_SIZE_MIN: usize = 1;
const ELASTICSEARCH_BATCH_SIZE_MAX: usize = 1000;
const MAX_SENTENCES_MIN: u32 = 1;
const MAX_SENTENCES_MAX: u32 = 1000;
const SENTENCES_PER_PARAGRAPH_MIN: u32 = 1;
const SENTENCES_PER_PARAGRAPH_MAX: u32 = 100;
const KNN_CANDIDATES_MULTIPLIER_MIN: u32 = 1;
const KNN_CANDIDATES_MULTIPLIER_MAX: u32 = 100;

trait SettingsUi {
    fn get_elasticsearch_url_str(&self) -> String;
    fn get_tika_url_str(&self) -> String;
    fn get_nnserver_url_str(&self) -> String;
    fn get_open_on_start(&self) -> bool;
    fn get_indexing_directories_dir_items(&self) -> Vec<DirectoryItem>;
    fn get_exclude_file_regex(&self) -> String;
    fn get_watcher_enabled(&self) -> bool;
    fn get_debouncer_timeout_str(&self) -> String;
    fn get_max_file_size_str(&self) -> String;
    fn get_nnserver_batch_size_str(&self) -> String;
    fn get_elasticsearch_batch_size_str(&self) -> String;
    fn get_max_sentences_str(&self) -> String;
    fn get_sentences_per_paragraph_str(&self) -> String;
    fn get_knn_candidates_multiplier_str(&self) -> String;

    fn valid_elasticsearch_url(elasticsearch_url_str: &str) -> bool;
    fn valid_tika_url(tika_url_str: &str) -> bool;
    fn valid_nnserver_url(nnserver_url_str: &str) -> bool;
    fn valid_debouncer_timeout(debouncer_timeout_str: &str) -> bool;
    fn valid_max_file_size(max_file_size_str: &str) -> bool;
    fn valid_nnserver_batch_size(nnserver_batch_size_str: &str) -> bool;
    fn valid_elasticsearch_batch_size(elasticsearch_batch_size_str: &str) -> bool;
    fn valid_max_sentences(max_sentences_str: &str) -> bool;
    fn valid_sentences_per_paragraph(sentences_per_paragraph_str: &str) -> bool;
    fn valid_knn_candidates_multiplier(knn_candidates_multiplier_str: &str) -> bool;

    #[allow(clippy::too_many_arguments)]
    fn parse(
        elasticsearch_url_str: &str,
        tika_url_str: &str,
        nnserver_url_str: &str,
        open_on_start: bool,
        indexing_directories_dir_items: &[DirectoryItem],
        exclude_file_regex: &str,
        watcher_enabled: bool,
        debouncer_timeout_str: &str,
        max_file_size_str: &str,
        nnserver_batch_size_str: &str,
        elasticsearch_batch_size_str: &str,
        max_sentences_str: &str,
        sentences_per_paragraph_str: &str,
        knn_candidates_multiplier_str: &str,
    ) -> Self;
}

impl SettingsUi for Settings {
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
    fn get_nnserver_batch_size_str(&self) -> String {
        self.nnserver_batch_size.to_string()
    }
    fn get_elasticsearch_batch_size_str(&self) -> String {
        self.elasticsearch_batch_size.to_string()
    }
    fn get_max_sentences_str(&self) -> String {
        self.max_sentences.to_string()
    }
    fn get_sentences_per_paragraph_str(&self) -> String {
        self.sentences_per_paragraph.to_string()
    }
    fn get_knn_candidates_multiplier_str(&self) -> String {
        self.knn_candidates_multiplier.to_string()
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
    fn valid_nnserver_batch_size(nnserver_batch_size_str: &str) -> bool {
        nnserver_batch_size_str
            .parse()
            .map(|x: usize| (NNSERVER_BATCH_SIZE_MIN..=NNSERVER_BATCH_SIZE_MAX).contains(&x))
            == Ok(true)
    }
    fn valid_elasticsearch_batch_size(elasticsearch_batch_size_str: &str) -> bool {
        elasticsearch_batch_size_str.parse().map(|x: usize| {
            (ELASTICSEARCH_BATCH_SIZE_MIN..=ELASTICSEARCH_BATCH_SIZE_MAX).contains(&x)
        }) == Ok(true)
    }
    fn valid_max_sentences(max_sentences_str: &str) -> bool {
        max_sentences_str
            .parse()
            .map(|x: u32| (MAX_SENTENCES_MIN..=MAX_SENTENCES_MAX).contains(&x))
            == Ok(true)
    }
    fn valid_sentences_per_paragraph(sentences_per_paragraph_str: &str) -> bool {
        sentences_per_paragraph_str
            .parse()
            .map(|x: u32| (SENTENCES_PER_PARAGRAPH_MIN..=SENTENCES_PER_PARAGRAPH_MAX).contains(&x))
            == Ok(true)
    }
    fn valid_knn_candidates_multiplier(knn_candidates_multiplier_str: &str) -> bool {
        knn_candidates_multiplier_str.parse().map(|x: u32| {
            (KNN_CANDIDATES_MULTIPLIER_MIN..=KNN_CANDIDATES_MULTIPLIER_MAX).contains(&x)
        }) == Ok(true)
    }

    fn parse(
        elasticsearch_url_str: &str,
        tika_url_str: &str,
        nnserver_url_str: &str,
        open_on_start: bool,
        indexing_directories_dir_items: &[DirectoryItem],
        exclude_file_regex: &str,
        watcher_enabled: bool,
        debouncer_timeout_str: &str,
        max_file_size_str: &str,
        nnserver_batch_size_str: &str,
        elasticsearch_batch_size_str: &str,
        max_sentences_str: &str,
        sentences_per_paragraph_str: &str,
        knn_candidates_multiplier_str: &str,
    ) -> Self {
        Self {
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
            nnserver_batch_size: nnserver_batch_size_str.parse().unwrap(),
            elasticsearch_batch_size: elasticsearch_batch_size_str.parse().unwrap(),
            max_sentences: max_sentences_str.parse().unwrap(),
            sentences_per_paragraph: sentences_per_paragraph_str.parse().unwrap(),
            knn_candidates_multiplier: knn_candidates_multiplier_str.parse().unwrap(),
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
    let nnserver_batch_size_str = create_signal(cx, settings.get().get_nnserver_batch_size_str());
    let elasticsearch_batch_size_str =
        create_signal(cx, settings.get().get_elasticsearch_batch_size_str());
    let max_sentences_str = create_signal(cx, settings.get().get_max_sentences_str());
    let sentences_per_paragraph_str =
        create_signal(cx, settings.get().get_sentences_per_paragraph_str());
    let knn_candidates_multiplier_str =
        create_signal(cx, settings.get().get_knn_candidates_multiplier_str());

    // Validation values for settings
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
    let nnserver_batch_size_valid = create_memo(cx, || {
        Settings::valid_nnserver_batch_size(&nnserver_batch_size_str.get())
    });
    let elasticsearch_batch_size_valid = create_memo(cx, || {
        Settings::valid_elasticsearch_batch_size(&elasticsearch_batch_size_str.get())
    });
    let max_sentences_valid = create_memo(cx, || {
        Settings::valid_max_sentences(&max_sentences_str.get())
    });
    let sentences_per_paragraph_valid = create_memo(cx, || {
        Settings::valid_sentences_per_paragraph(&sentences_per_paragraph_str.get())
    });
    let knn_candidates_multiplier_valid = create_memo(cx, || {
        Settings::valid_knn_candidates_multiplier(&knn_candidates_multiplier_str.get())
    });
    let any_invalid = create_memo(cx, || {
        !*elasticsearch_url_valid.get()
            || !*tika_url_valid.get()
            || !*nnserver_url_valid.get()
            || !*debouncer_timeout_valid.get()
            || !*max_file_size_valid.get()
            || !*nnserver_batch_size_valid.get()
            || !*elasticsearch_batch_size_valid.get()
            || !*max_sentences_valid.get()
            || !*sentences_per_paragraph_valid.get()
            || !*knn_candidates_multiplier_valid.get()
    });

    // Set input values from settings when they are updated (on load from server or reset)
    create_effect(cx, || {
        elasticsearch_url_str.set(settings.get().get_elasticsearch_url_str());
        tika_url_str.set(settings.get().get_tika_url_str());
        nnserver_url_str.set(settings.get().get_nnserver_url_str());
        open_on_start.set(settings.get().get_open_on_start());
        indexing_directories.set(settings.get().get_indexing_directories_dir_items());
        exclude_file_regex.set(settings.get().get_exclude_file_regex());
        max_file_size_str.set(settings.get().get_max_file_size_str());
        nnserver_batch_size_str.set(settings.get().get_nnserver_batch_size_str());
        elasticsearch_batch_size_str.set(settings.get().get_elasticsearch_batch_size_str());
        max_sentences_str.set(settings.get().get_max_sentences_str());
        sentences_per_paragraph_str.set(settings.get().get_sentences_per_paragraph_str());
        knn_candidates_multiplier_str.set(settings.get().get_knn_candidates_multiplier_str());
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
                &elasticsearch_url_str.get(),
                &tika_url_str.get(),
                &nnserver_url_str.get(),
                *open_on_start.get(),
                &indexing_directories.get(),
                &exclude_file_regex.get(),
                *watcher_enabled.get(),
                &debouncer_timeout_str.get(),
                &max_file_size_str.get(),
                &nnserver_batch_size_str.get(),
                &elasticsearch_batch_size_str.get(),
                &max_sentences_str.get(),
                &sentences_per_paragraph_str.get(),
                &knn_candidates_multiplier_str.get(),
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
                        NumberSetting(id="nnserver_batch_size", label="Максимальное количество одновременно обрабатываемых документов: ",
                            value=nnserver_batch_size_str, valid=nnserver_batch_size_valid)
                        NumberSetting(id="elasticsearch_batch_size", label="Количество отправляемых в Elasticsearch изменений за раз: ",
                            value=elasticsearch_batch_size_str, valid=elasticsearch_batch_size_valid)
                        NumberSetting(id="max_sentences", label="Максимальное количество предложений, обрабатываемых нейронной сетью: ",
                            value=max_sentences_str, valid=max_sentences_valid)
                        NumberSetting(id="sentences_per_paragraph", label="Количество предложений, обрабатываемых за один раз: ",
                            value=sentences_per_paragraph_str, valid=sentences_per_paragraph_valid)
                    }

                    fieldset {
                        legend { "Настройки поиска" }
                        NumberSetting(id="knn_candidates_multiplier", label="Множитель количества кандидатов kNN при семантическом поиске: ",
                            value=knn_candidates_multiplier_str, valid=knn_candidates_multiplier_valid)
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
