use crate::auth::{complete_device_login, load_accounts, request_device_code, save_accounts};
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Dialog, Label, Orientation};
use std::path::PathBuf;

fn accounts_path() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("BLauncher").join("accounts.json")
}

pub fn open_profiles_dialog(parent: Option<&gtk4::Window>) {
    let dialog = Dialog::builder().title("Profiles").modal(true).default_width(460).default_height(360).build();
    if let Some(parent) = parent { dialog.set_transient_for(Some(parent)); }

    let content = GtkBox::builder().orientation(Orientation::Vertical).spacing(12)
        .margin_top(18).margin_bottom(18).margin_start(18).margin_end(18).build();
    let title = Label::builder().label("Minecraft Accounts").halign(Align::Start).build();
    title.add_css_class("title-2");
    let status = Label::builder().label("Sign in with your Microsoft account to use Minecraft Java Edition.")
        .wrap(true).halign(Align::Start).build();
    status.add_css_class("dim-label");
    let accounts_box = GtkBox::new(Orientation::Vertical, 8);
    let login_button = Button::builder().label("Sign in with Microsoft").halign(Align::Fill).build();
    login_button.add_css_class("suggested-action");
    let close_button = Button::with_label("Close");
    content.append(&title); content.append(&status); content.append(&accounts_box);
    content.append(&login_button); content.append(&close_button); dialog.content_area().append(&content);

    let path = accounts_path();
    let accounts_box_load = accounts_box.clone();
    let rt = std::sync::Arc::new(tokio::runtime::Runtime::new().expect("tokio runtime"));
    rt.clone().spawn(async move {
        let accounts = load_accounts(&path).await.unwrap_or_default();
        let names: Vec<String> = accounts.into_iter().map(|a| a.name).collect();
        let _ = glib::MainContext::default().spawn_local(async move {
            while let Some(child) = accounts_box_load.first_child() { accounts_box_load.remove(&child); }
            if names.is_empty() { accounts_box_load.append(&Label::new(Some("No Microsoft accounts added yet."))); }
            else { for name in names { accounts_box_load.append(&Label::builder().label(&format!("✓  {}", name)).halign(Align::Start).build()); } }
        });
    });

    let path_login = accounts_path();
    let status_click = status.clone();
    let login_click = login_button.clone();
    let accounts_box_click = accounts_box.clone();
    login_button.connect_clicked(move |_| {
        login_click.set_sensitive(false);
        status_click.set_label("Requesting a Microsoft sign-in code…");
        let status_device = status_click.clone();
        let status_result = status_click.clone();
        let button_device = login_click.clone();
        let button_result = login_click.clone();
        let accounts_box_device = accounts_box_click.clone();
        let accounts_box_result = accounts_box_click.clone();
        let path_result = path_login.clone();
        let rt_device = rt.clone();
        rt_device.spawn(async move {
            match request_device_code().await {
                Ok(device) => {
                    let code = device.user_code.clone(); let url = device.verification_uri.clone();
                    let _ = glib::MainContext::default().spawn_local(async move {
                        status_device.set_markup(&format!("Open <b>{}</b> and enter code <b>{}</b>.", url, code));
                        let open = Button::with_label("Open Microsoft sign-in page");
                        let url_clone = url.clone();
                        open.connect_clicked(move |_| { let _ = open::that(&url_clone); });
                        accounts_box_device.append(&open);
                    });
                    let result = complete_device_login(device).await;
                    let _ = glib::MainContext::default().spawn_local(async move {
                        match result {
                            Ok(account) => {
                                let mut accounts = load_accounts(&path_result).await.unwrap_or_default();
                                accounts.retain(|a| a.id != account.id); accounts.push(account.clone());
                                if let Err(error) = save_accounts(&path_result, &accounts).await {
                                    status_result.set_label(&format!("Login succeeded, but saving failed: {}", error));
                                } else {
                                    status_result.set_label(&format!("Signed in as {}.", account.name));
                                    accounts_box_result.append(&Label::builder().label(&format!("✓  {}", account.name)).halign(Align::Start).build());
                                }
                            }
                            Err(error) => status_result.set_label(&format!("Microsoft login failed: {}", error)),
                        }
                        button_result.set_sensitive(true);
                    });
                }
                Err(error) => {
                    let _ = glib::MainContext::default().spawn_local(async move {
                        status_result.set_label(&format!("Could not start Microsoft login: {}", error));
                        button_result.set_sensitive(true);
                    });
                }
            }
        });
    });

    let dialog_clone = dialog.clone();
    close_button.connect_clicked(move |_| dialog_clone.close());
    dialog.present();
}
