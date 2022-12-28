use std::path::PathBuf;

use common_lib::settings::{ClientSettings, ServerSettings};
use serde::Serialize;
use serde_wasm_bindgen::{from_value, to_value};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use url::Url;
use uuid::Uuid;

use crate::app::{invoke, widgets::StatusDialogState};

pub const MAX_FILE_SIZE_MIN: f64 = 0.01;
pub const MAX_FILE_SIZE_MAX: f64 = 1000.0;

trait ClientSettingsUi {
    fn get_indexer_url_str(&self) -> String;

    fn valid_indexer_url(indexer_url_str: &str) -> bool;

    fn parse(indexer_url_str: &str) -> Self;
}

trait ServerSettingsUi {
    fn get_elasticsearch_url_str(&self) -> String;
    fn get_tika_url_str(&self) -> String;
    fn get_nnserver_url_str(&self) -> String;
    fn get_indexing_directories_dir_items(&self) -> Vec<DirectoryItem>;
    fn get_max_file_size_str(&self) -> String;

    fn valid_elasticsearch_url(elasticsearch_url_str: &str) -> bool;
    fn valid_tika_url(tika_url_str: &str) -> bool;
    fn valid_nnserver_url(nnserver_url_str: &str) -> bool;
    fn valid_max_file_size(max_file_size_str: &str) -> bool;

    fn parse(
        elasticsearch_url_str: &str,
        tika_url_str: &str,
        nnserver_url_str: &str,
        indexing_directories_dir_items: &[DirectoryItem],
        max_file_size_str: &str,
    ) -> Self;
}

impl ClientSettingsUi for ClientSettings {
    fn get_indexer_url_str(&self) -> String {
        self.indexer_url.to_string()
    }

    fn valid_indexer_url(indexer_url_str: &str) -> bool {
        Url::parse(indexer_url_str).is_ok()
    }

    fn parse(indexer_url_str: &str) -> Self {
        Self {
            indexer_url: Url::parse(indexer_url_str).unwrap(),
        }
    }
}

impl ServerSettingsUi for ServerSettings {
    fn get_elasticsearch_url_str(&self) -> String {
        self.elasticsearch_url.to_string()
    }
    fn get_tika_url_str(&self) -> String {
        self.tika_url.to_string()
    }
    fn get_nnserver_url_str(&self) -> String {
        self.nnserver_url.to_string()
    }
    fn get_indexing_directories_dir_items(&self) -> Vec<DirectoryItem> {
        self.indexing_directories
            .iter()
            .map(|p| DirectoryItem::new(p.clone()))
            .collect()
    }
    fn get_max_file_size_str(&self) -> String {
        ((self.max_file_size as f64) / 1024.0 / 1024.0).to_string()
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
    fn valid_max_file_size(max_file_size_str: &str) -> bool {
        max_file_size_str
            .parse()
            .map(|x: f64| (MAX_FILE_SIZE_MIN..=MAX_FILE_SIZE_MAX).contains(&x))
            == Ok(true)
    }

    fn parse(
        elasticsearch_url_str: &str,
        tika_url_str: &str,
        nnserver_url_str: &str,
        indexing_directories_dir_items: &[DirectoryItem],
        max_file_size_str: &str,
    ) -> Self {
        Self {
            elasticsearch_url: Url::parse(elasticsearch_url_str).unwrap(),
            tika_url: Url::parse(tika_url_str).unwrap(),
            nnserver_url: Url::parse(nnserver_url_str).unwrap(),
            indexing_directories: indexing_directories_dir_items
                .iter()
                .map(|f| f.path.clone())
                .collect(),
            max_file_size: (max_file_size_str.parse::<f64>().unwrap() * 1024.0 * 1024.0) as u64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirectoryItem {
    id: Uuid,
    path: PathBuf,
}

impl DirectoryItem {
    fn new(path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            path,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetClientSettingsArgs<'a> {
    client_settings: &'a ClientSettings,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetServerSettingsArgs<'a> {
    server_settings: &'a ServerSettings,
}

#[component(inline_props)]
pub fn Settings<'a, G: Html>(
    cx: Scope<'a>,
    client_settings: &'a Signal<ClientSettings>,
    server_settings: &'a Signal<ServerSettings>,
    status_dialog_state: &'a Signal<StatusDialogState>,
) -> View<G> {
    let server_loaded = create_signal(cx, false);

    // Input values for settings
    let indexer_url_str = create_signal(cx, client_settings.get().get_indexer_url_str());
    let elasticsearch_url_str =
        create_signal(cx, server_settings.get().get_elasticsearch_url_str());
    let tika_url_str = create_signal(cx, server_settings.get().get_tika_url_str());
    let nnserver_url_str = create_signal(cx, server_settings.get().get_nnserver_url_str());
    let indexing_directories = create_signal(
        cx,
        server_settings.get().get_indexing_directories_dir_items(),
    );
    let max_file_size_str = create_signal(cx, server_settings.get().get_max_file_size_str());

    // Validation values for settings
    let indexer_url_valid = create_memo(cx, || {
        ClientSettings::valid_indexer_url(indexer_url_str.get().as_str())
    });
    let elasticsearch_url_valid = create_memo(cx, || {
        ServerSettings::valid_elasticsearch_url(elasticsearch_url_str.get().as_str())
    });
    let tika_url_valid = create_memo(cx, || {
        ServerSettings::valid_tika_url(tika_url_str.get().as_str())
    });
    let nnserver_url_valid = create_memo(cx, || {
        ServerSettings::valid_nnserver_url(nnserver_url_str.get().as_str())
    });
    let max_file_size_valid = create_memo(cx, || {
        ServerSettings::valid_max_file_size(max_file_size_str.get().as_str())
    });
    let any_invalid = create_memo(cx, || {
        !*indexer_url_valid.get()
            || !*elasticsearch_url_valid.get()
            || !*tika_url_valid.get()
            || !*nnserver_url_valid.get()
            || !*max_file_size_valid.get()
    });

    // Set input values from settings when they are updated (on load from server or reset)
    create_effect(cx, || {
        indexer_url_str.set(client_settings.get().get_indexer_url_str())
    });
    create_effect(cx, || {
        elasticsearch_url_str.set(server_settings.get().get_elasticsearch_url_str());
        tika_url_str.set(server_settings.get().get_tika_url_str());
        nnserver_url_str.set(server_settings.get().get_nnserver_url_str());
        indexing_directories.set(server_settings.get().get_indexing_directories_dir_items());
        max_file_size_str.set(server_settings.get().get_max_file_size_str());
    });
    let reset_settings = |_| {
        client_settings.trigger_subscribers();
        server_settings.trigger_subscribers();
    };

    let load_client_settings = move || async move {
        match invoke("get_client_settings", wasm_bindgen::JsValue::UNDEFINED)
            .await
            .map_err(|e| e.as_string().unwrap())
            .and_then(|x| from_value(x).map_err(|e| e.to_string()))
        {
            Ok(x) => {
                client_settings.set(x);
                true
            }
            Err(e) => {
                status_dialog_state.set(StatusDialogState::Error(
                    "❌ Ошибка загрузки клиентских настроек: ".to_owned() + &e,
                ));
                false
            }
        }
    };
    let load_server_settings = move || async move {
        match invoke("get_server_settings", wasm_bindgen::JsValue::UNDEFINED)
            .await
            .map_err(|e| e.as_string().unwrap())
            .and_then(|x| from_value(x).map_err(|e| e.to_string()))
        {
            Ok(x) => {
                server_settings.set(x);
                server_loaded.set(true);
                true
            }
            Err(e) => {
                status_dialog_state.set(StatusDialogState::Error(
                    "❌ Ошибка загрузки серверных настроек: ".to_owned() + &e,
                ));
                false
            }
        }
    };

    // Load settings
    spawn_local_scoped(cx, async move {
        status_dialog_state.set(StatusDialogState::Loading);

        if load_client_settings().await && load_server_settings().await {
            status_dialog_state.set(StatusDialogState::None);
        }
    });

    // Save settings
    let set_settings = move |_| {
        spawn_local_scoped(cx, async move {
            status_dialog_state.set(StatusDialogState::Loading);

            let new_client_settings = ClientSettings::parse(&indexer_url_str.get());

            if let Err(e) = invoke(
                "set_client_settings",
                to_value(&SetClientSettingsArgs {
                    client_settings: &new_client_settings,
                })
                .unwrap(),
            )
            .await
            {
                status_dialog_state.set(StatusDialogState::Error(
                    "❌ Ошибка сохранения клиентских настроек: ".to_owned()
                        + &e.as_string().unwrap(),
                ));
                return;
            }

            client_settings.set(new_client_settings);

            if *server_loaded.get() {
                let new_server_settings = ServerSettings::parse(
                    &elasticsearch_url_str.get(),
                    &tika_url_str.get(),
                    &nnserver_url_str.get(),
                    &indexing_directories.get(),
                    &max_file_size_str.get(),
                );

                if let Err(e) = invoke(
                    "set_server_settings",
                    to_value(&SetServerSettingsArgs {
                        server_settings: &new_server_settings,
                    })
                    .unwrap(),
                )
                .await
                {
                    status_dialog_state.set(StatusDialogState::Error(
                        "❌ Ошибка сохранения серверных настроек: ".to_owned()
                            + &e.as_string().unwrap(),
                    ));
                    return;
                }

                server_settings.set(new_server_settings);
            } else if !load_server_settings().await {
                return;
            };

            status_dialog_state.set(StatusDialogState::Info("✅ Настройки сохранены".to_owned()));
        })
    };

    view! { cx,
        div(class="main_container") {
            main {
                form(id="settings", on:submit=set_settings, action="javascript:void(0);") {
                    fieldset {
                        legend { "Клиентские настройки" }
                        TextSetting(id="indexer_url", label="URL сервера индексации: ",
                            value=indexer_url_str, valid=indexer_url_valid)
                    }

                    (if *server_loaded.get() {
                        view! { cx,
                            fieldset {
                                legend { "Серверные настройки" }
                                TextSetting(id="elasticsearch_url", label="URL сервера Elasticsearch: ",
                                    value=elasticsearch_url_str, valid=elasticsearch_url_valid)
                                TextSetting(id="tika_url", label="URL сервера Apache Tika: ",
                                    value=tika_url_str, valid=tika_url_valid)
                                TextSetting(id="nnserver_url", label="URL сервера нейронных сетей: ",
                                    value=nnserver_url_str, valid=nnserver_url_valid)
                            }

                            fieldset {
                                legend { "Индексируемые папки" }
                                DirectoryList(directory_list=indexing_directories)
                            }

                            fieldset {
                                legend { "Настройки индексации" }
                                NumberSetting(id="max_file_size", label="Максимальный размер файла (МиБ): ",
                                    value=max_file_size_str, valid=max_file_size_valid)
                            }
                        }
                    } else {
                        view! {cx, }
                    })

                    div(class="settings_buttons") {
                        button(type="button", on:click=reset_settings) { "Отмена" }
                        button(type="submit", disabled=*any_invalid.get()) { "Сохранить" }
                    }
                }
            }
        }
    }
}

#[derive(Prop)]
struct TextSettingProps<'a> {
    id: &'static str,
    label: &'static str,
    value: &'a Signal<String>,
    valid: &'a ReadSignal<bool>,
}

#[component]
fn TextSetting<'a, G: Html>(cx: Scope<'a>, props: TextSettingProps<'a>) -> View<G> {
    let value = props.value;
    view! { cx,
        div(class="setting") {
            label(for=props.id) { (props.label) }
            input(type="text", id=props.id, name=props.id, bind:value=value) {}
            (if *props.valid.get() { "✅" } else { "❌" })
        }
    }
}

#[derive(Prop)]
struct NumberSettingProps<'a> {
    id: &'static str,
    label: &'static str,
    value: &'a Signal<String>,
    valid: &'a ReadSignal<bool>,
}

#[component]
fn NumberSetting<'a, G: Html>(cx: Scope<'a>, props: NumberSettingProps<'a>) -> View<G> {
    let value = props.value;
    view! { cx,
        div(class="setting") {
            label(for=props.id) { (props.label) }
            input(type="text", size=10, id=props.id, name=props.id, bind:value=value) {}
            (if *props.valid.get() { "✅" } else { "❌" })
        }
    }
}

#[component(inline_props)]
fn DirectoryList<'a, G: Html>(
    cx: Scope<'a>,
    directory_list: &'a Signal<Vec<DirectoryItem>>,
) -> View<G> {
    let curr_directory = create_signal(cx, PathBuf::new());
    let curr_directory_empty = create_memo(cx, || curr_directory.get().as_os_str().is_empty());

    let select_item = move |_| {
        spawn_local_scoped(cx, async {
            let path_option: Option<PathBuf> = from_value(
                invoke("pick_folder", wasm_bindgen::JsValue::UNDEFINED)
                    .await
                    .unwrap(),
            )
            .unwrap();

            if let Some(path) = path_option {
                curr_directory.set(path);
            }
        });
    };

    let add_item = |_| {
        directory_list
            .modify()
            .push(DirectoryItem::new((*curr_directory.get()).clone()));
        curr_directory.set(PathBuf::new());
    };

    view! { cx,
        Keyed(
            iterable=directory_list,
            view=move |cx, item| {
                let delete_item = move |_| {
                    directory_list.modify().retain(|x| x.id != item.id);
                };

                view! { cx,
                    div(class="setting") {
                        input(type="text", readonly=true, value=item.path.display()) {}
                        button(type="button", on:click=delete_item) { "-" }
                    }
                }
            },
            key=|item| item.id,
        )

        div(class="setting") {
            input(type="text", readonly=true, value=curr_directory.get().display()) {}
            button(type="button", on:click=select_item) { "Выбрать..." }
            button(type="button", on:click=add_item, disabled=*curr_directory_empty.get()) { "+" }
        }
    }
}
