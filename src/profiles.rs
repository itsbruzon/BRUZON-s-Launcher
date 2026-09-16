use crate::auth::{complete_device_login, load_accounts, request_device_code, save_accounts};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Dialog, Label, Orientation};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn accounts_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("BLauncher")
        .join("accounts.json")
}

pub fn open_profiles_dialog(parent: Option<&gtk4::Window>) {
    let dialog = Dialog::builder()
        .title("Profiles")
        .modal(true)
        .default_width(460)
        .default_height(360)
        .build();
    if let Some(parent) = parent {
        dialog.set_transient_for(Some(parent));
    }

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    let title = Label::builder()
        .label("Minecraft Accounts")
        .halign(Align::Start)
        .build();
    title.add_css_class("title-2");
    let status = Label::builder()
        .label("Sign in with your Microsoft account to use Minecraft Java Edition.")
        .wrap(true)
        .halign(Align::Start)
        .build();
    status.add_css_class("dim-label");
    let accounts_box = GtkBox::new(Orientation::Vertical, 8);
    let login_button = Button::builder()
        .label("Sign in with Microsoft")
        .halign(Align::Fill)
        .build();
    login_button.add_css_class("suggested-action");
    let close_button = Button::with_label("Close");

    content.append(&title);
    content.append(&status);
    content.append(&accounts_box);
    content.append(&login_button);
    content.append(&close_button);
    dialog.content_area().append(&content);

    // Tokio is used only for async filesystem/network work. GTK objects are
    // strictly kept on the GLib main thread because they are !Send/!Sync.
    let rt = Arc::new(Runtime::new().expect("tokio runtime"));

    let path_initial = accounts_path();
    let accounts_box_initial = accounts_box.clone();
    let rt_initial = rt.clone();
    let initial_handle = rt_initial.spawn(async move {
        let accounts = load_accounts(&path_initial).await.unwrap_or_default();
        accounts.into_iter().map(|a| a.name).collect::<Vec<_>>()
    });

    glib::MainContext::default().spawn_local(async move {
        let names = initial_handle.await.unwrap_or_default();
        while let Some(child) = accounts_box_initial.first_child() {
            accounts_box_initial.remove(&child);
        }
        if names.is_empty() {
            accounts_box_initial.append(&Label::new(Some(
                "No Microsoft accounts added yet.",
            )));
        } else {
            for name in names {
                accounts_box_initial.append(
                    &Label::builder()
                        .label(&format!("✓  {}", name))
                        .halign(Align::Start)
                        .build(),
                );
            }
        }
    });

    let path_login = accounts_path();
    let status_click = status.clone();
    let login_click = login_button.clone();
    let accounts_box_click = accounts_box.clone();
    let rt_login = rt.clone();

    login_button.connect_clicked(move |_| {
        login_click.set_sensitive(false);
        status_click.set_label("Requesting a Microsoft sign-in code…");

        let status_ui = status_click.clone();
        let button_ui = login_click.clone();
        let accounts_box_ui = accounts_box_click.clone();
        let path_result = path_login.clone();
        let rt_network = rt_login.clone();

        // This task captures no GTK objects, so Tokio can safely move it to a
        // worker thread.
        let device_handle = rt_network.spawn(async move { request_device_code().await });

        glib::MainContext::default().spawn_local(async move {
            let device = match device_handle.await {
                Ok(Ok(device)) => device,
                Ok(Err(error)) => {
                    status_ui.set_label(&format!(
                        "Could not start Microsoft login: {}",
                        error
                    ));
                    button_ui.set_sensitive(true);
                    return;
                }
                Err(error) => {
                    status_ui.set_label(&format!(
                        "Microsoft login task failed: {}",
                        error
                    ));
                    button_ui.set_sensitive(true);
                    return;
                }
            };

            let code = device.user_code.clone();
            let url = device.verification_uri.clone();
            status_ui.set_label(&format!("Open {} and enter code {}.", url, code));

            let open = Button::with_label("Open Microsoft sign-in page");
            let url_for_button = url.clone();
            open.connect_clicked(move |_| {
                let _ = open::that(&url_for_button);
            });
            accounts_box_ui.append(&open);

            // Again, only Send data is captured by the Tokio task.
            let rt_auth = rt_login.clone();
            let auth_handle = rt_auth.spawn(async move { complete_device_login(device).await });
            let path_save = path_result.clone();

            let result = auth_handle.await;
            match result {
                Ok(Ok(account)) => {
                    let mut accounts = load_accounts(&path_save).await.unwrap_or_default();
                    accounts.retain(|a| a.id != account.id);
                    accounts.push(account.clone());

                    if let Err(error) = save_accounts(&path_save, &accounts).await {
                        status_ui.set_label(&format!(
                            "Login succeeded, but saving failed: {}",
                            error
                        ));
                    } else {
                        status_ui.set_label(&format!("Signed in as {}.", account.name));
                        accounts_box_ui.append(
                            &Label::builder()
                                .label(&format!("✓  {}", account.name))
                                .halign(Align::Start)
                                .build(),
                        );
                    }
                }
                Ok(Err(error)) => {
                    status_ui.set_label(&format!("Microsoft login failed: {}", error));
                }
                Err(error) => {
                    status_ui.set_label(&format!("Microsoft login task failed: {}", error));
                }
            }

            button_ui.set_sensitive(true);
        });
    });

    let dialog_clone = dialog.clone();
    close_button.connect_clicked(move |_| dialog_clone.close());
    dialog.present();
}
