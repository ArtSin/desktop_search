use sycamore::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlDialogElement;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusDialogState {
    None,
    Loading,
    Info(String),
    Error(String),
}

#[component(inline_props)]
pub fn StatusDialog<'a, G: Html>(
    cx: Scope<'a>,
    status: &'a ReadSignal<StatusDialogState>,
) -> View<G> {
    let header_str = create_memo(cx, || match *status.get() {
        StatusDialogState::None | StatusDialogState::Loading => String::new(),
        StatusDialogState::Info(_) => "Информация".to_owned(),
        StatusDialogState::Error(_) => "Ошибка".to_owned(),
    });
    let message_str = create_memo(cx, || match *status.get() {
        StatusDialogState::None => String::new(),
        StatusDialogState::Loading => "⏳ Загрузка...".to_owned(),
        StatusDialogState::Info(ref x) => x.clone(),
        StatusDialogState::Error(ref x) => x.clone(),
    });
    let show_dialog = create_memo(cx, || !message_str.get().is_empty());
    create_effect(cx, || {
        show_dialog.track();
        if let Some(element) = web_sys::window()
            .expect("`window` not found")
            .document()
            .expect("`document` not found")
            .get_element_by_id("dialog")
        {
            let dialog: HtmlDialogElement =
                element.dyn_into().expect("`dialog` has incorrect type");

            if dialog.open() == *show_dialog.get() {
                return;
            }
            if *show_dialog.get() {
                dialog.show_modal().expect("Can't open dialog");
            } else {
                dialog.close();
            }
        }
    });

    view! { cx,
        dialog(id="dialog") {
            (if !header_str.get().is_empty() {
                view! { cx,
                    header { (header_str.get()) }
                }
            } else {
                view! { cx, }
            })

            form(method="dialog") {
                p {
                    (message_str.get())
                }

                (if *status.get() != StatusDialogState::Loading {
                    view! { cx,
                        menu {
                            button { "ОК" }
                        }
                    }
                } else {
                    view! { cx, }
                })
            }
        }
    }
}
