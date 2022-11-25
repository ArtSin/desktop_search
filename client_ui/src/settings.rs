use common_lib::{ClientSettings, ServerSettings};
use serde::Serialize;
use serde_wasm_bindgen::{from_value, to_value};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use url::Url;

use crate::app::invoke;

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

#[component]
pub fn Settings<G: Html>(cx: Scope) -> View<G> {
    let client_settings = create_signal(cx, ClientSettings::default());
    let server_settings = create_signal(cx, ServerSettings::default());

    let server_loaded = create_signal(cx, false);
    let status_str = create_signal(cx, String::new());

    let indexer_url_str = create_signal(cx, client_settings.get().indexer_url.to_string());
    let elasticsearch_url_str =
        create_signal(cx, server_settings.get().elasticsearch_url.to_string());
    let tika_url_str = create_signal(cx, server_settings.get().tika_url.to_string());
    let nnserver_url_str = create_signal(cx, server_settings.get().nnserver_url.to_string());

    let indexer_url_valid = create_memo(cx, || Url::parse(indexer_url_str.get().as_str()).is_ok());
    let elasticsearch_url_valid = create_memo(cx, || {
        Url::parse(elasticsearch_url_str.get().as_str()).is_ok()
    });
    let tika_url_valid = create_memo(cx, || Url::parse(tika_url_str.get().as_str()).is_ok());
    let nnserver_url_valid =
        create_memo(cx, || Url::parse(nnserver_url_str.get().as_str()).is_ok());
    let any_invalid = create_memo(cx, || {
        !*indexer_url_valid.get()
            || !*elasticsearch_url_valid.get()
            || !*tika_url_valid.get()
            || !*nnserver_url_valid.get()
    });

    create_effect(cx, || {
        indexer_url_str.set(client_settings.get().indexer_url.to_string())
    });
    create_effect(cx, || {
        elasticsearch_url_str.set(server_settings.get().elasticsearch_url.to_string());
        tika_url_str.set(server_settings.get().tika_url.to_string());
        nnserver_url_str.set(server_settings.get().nnserver_url.to_string());
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
                status_str.set("❌ Ошибка загрузки клиентских настроек: ".to_owned() + &e);
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
                status_str.set("❌ Ошибка загрузки серверных настроек: ".to_owned() + &e);
                false
            }
        }
    };

    spawn_local_scoped(cx, async move {
        status_str.set("⏳ Загрузка...".to_owned());

        if load_client_settings().await && load_server_settings().await {
            status_str.set("".to_owned());
        }
    });

    let set_settings = move |_| {
        spawn_local_scoped(cx, async move {
            status_str.set("⏳ Сохранение...".to_owned());

            let new_client_settings = ClientSettings {
                indexer_url: Url::parse(&indexer_url_str.get()).unwrap(),
            };

            if let Err(e) = invoke(
                "set_client_settings",
                to_value(&SetClientSettingsArgs {
                    client_settings: &new_client_settings,
                })
                .unwrap(),
            )
            .await
            {
                status_str.set(
                    "❌ Ошибка сохранения клиентских настроек: ".to_owned()
                        + &e.as_string().unwrap(),
                );
                return;
            }

            client_settings.set(new_client_settings);

            if *server_loaded.get() {
                let new_server_settings = ServerSettings {
                    elasticsearch_url: Url::parse(&elasticsearch_url_str.get()).unwrap(),
                    tika_url: Url::parse(&tika_url_str.get()).unwrap(),
                    nnserver_url: Url::parse(&nnserver_url_str.get()).unwrap(),
                };

                if let Err(e) = invoke(
                    "set_server_settings",
                    to_value(&SetServerSettingsArgs {
                        server_settings: &new_server_settings,
                    })
                    .unwrap(),
                )
                .await
                {
                    status_str.set(
                        "❌ Ошибка сохранения серверных настроек: ".to_owned()
                            + &e.as_string().unwrap(),
                    );
                    return;
                }

                server_settings.set(new_server_settings);
            } else if !load_server_settings().await {
                return;
            };

            status_str.set("✅ Настройки сохранены".to_owned());
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
                        }
                    } else {
                        view! {cx, }
                    })

                    div(class="settings_buttons") {
                        button(type="button", on:click=reset_settings) { "Отмена" }
                        button(type="submit", disabled=*any_invalid.get()) { "Сохранить" }
                    }
                }

                (if !status_str.get().is_empty() {
                    view! { cx,
                        p(class="status") {
                            (status_str.get())
                        }
                    }
                } else {
                    view! { cx, }
                })
            }
        }
    }
}

#[derive(Props)]
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
        div(class="text_setting") {
            label(for=props.id) { (props.label) }
            input(type="text", id=props.id, name=props.id, bind:value=value) {}
            (if *props.valid.get() { "✅" } else { "❌" })
        }
    }
}
