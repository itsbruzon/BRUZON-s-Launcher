use crate::auth::{
    accounts_path, complete_device_login, load_accounts, load_selected_account, offline_account,
    request_device_code, save_accounts, save_selected_account, selected_account_path, AccountType,
    MinecraftAccount,
};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Entry, Label, Orientation, Separator};
use relm4::gtk;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub fn create_profiles_page(rt: Arc<Runtime>) -> gtk::ScrolledWindow {
    let root = GtkBox::new(Orientation::Vertical, 18);
    root.set_margin_top(28);
    root.set_margin_bottom(28);
    root.set_margin_start(32);
    root.set_margin_end(32);

    let title = Label::new(Some("Profiles"));
    title.add_css_class("title-1");
    title.set_halign(Align::Start);
    root.append(&title);

    let subtitle = Label::new(Some(
        "Manage the accounts that can be used to launch your Minecraft instances.",
    ));
    subtitle.add_css_class("dim-label");
    subtitle.set_wrap(true);
    subtitle.set_halign(Align::Start);
    root.append(&subtitle);

    let account_area = GtkBox::new(Orientation::Vertical, 10);
    account_area.set_hexpand(true);
    root.append(&account_area);

    let status = Label::new(None);
    status.add_css_class("dim-label");
    status.set_wrap(true);
    status.set_selectable(true);
    status.set_halign(Align::Start);
    root.append(&status);

    let actions = GtkBox::new(Orientation::Horizontal, 8);
    let add_offline = Button::with_label("Add Offline Account");
    let add_microsoft = Button::with_label("Sign in with Microsoft");
    add_microsoft.add_css_class("suggested-action");
    actions.append(&add_offline);
    actions.append(&add_microsoft);
    root.append(&actions);

    let device_area = GtkBox::new(Orientation::Vertical, 8);
    device_area.set_visible(false);
    device_area.add_css_class("card");
    let device_label = Label::new(Some("Microsoft sign-in code"));
    device_label.add_css_class("title-4");
    device_label.set_halign(Align::Start);
    let device_code = Entry::new();
    device_code.set_editable(false);
    device_code.set_halign(Align::Fill);
    device_code.add_css_class("title-2");
    let device_actions = GtkBox::new(Orientation::Horizontal, 8);
    let copy_code = Button::with_label("Copy code");
    let open_signin = Button::with_label("Open Microsoft sign-in");
    open_signin.add_css_class("suggested-action");
    device_actions.append(&copy_code);
    device_actions.append(&open_signin);
    device_area.append(&device_label);
    device_area.append(&device_code);
    device_area.append(&device_actions);
    root.append(&device_area);

    let accounts: Rc<RefCell<Vec<MinecraftAccount>>> = Rc::new(RefCell::new(Vec::new()));
    let selected: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let rebuild_slot: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    let rebuild: Rc<dyn Fn()> = Rc::new({
        let account_area = account_area.clone();
        let accounts = accounts.clone();
        let selected = selected.clone();
        let status = status.clone();
        let rt = rt.clone();
        let rebuild_slot = rebuild_slot.clone();
        move || {
            while let Some(child) = account_area.first_child() {
                account_area.remove(&child);
            }

            for account in accounts.borrow().clone() {
                let card = GtkBox::new(Orientation::Vertical, 8);
                card.set_hexpand(true);
                card.add_css_class("card");

                let header = GtkBox::new(Orientation::Horizontal, 10);
                let avatar = Label::new(Some(if account.account_type == AccountType::Microsoft {
                    "☁"
                } else {
                    "👤"
                }));
                avatar.set_size_request(44, 44);
                avatar.add_css_class("title-2");
                header.append(&avatar);

                let info = GtkBox::new(Orientation::Vertical, 2);
                info.set_hexpand(true);
                let name = Label::new(Some(&account.name));
                name.add_css_class("title-4");
                name.set_halign(Align::Start);
                let kind = Label::new(Some(match account.account_type {
                    AccountType::Microsoft => "Microsoft account",
                    AccountType::Offline => "Offline account",
                }));
                kind.add_css_class("dim-label");
                kind.set_halign(Align::Start);
                info.append(&name);
                info.append(&kind);
                header.append(&info);
                card.append(&header);

                let controls = GtkBox::new(Orientation::Horizontal, 8);
                let is_selected = selected.borrow().as_deref() == Some(account.id.as_str());
                let select = Button::with_label(if is_selected { "Selected" } else { "Use Account" });
                if is_selected {
                    select.set_sensitive(false);
                } else {
                    select.add_css_class("suggested-action");
                }

                let account_id = account.id.clone();
                let selected_for_select = selected.clone();
                let status_for_select = status.clone();
                let rt_for_select = rt.clone();
                select.connect_clicked(move |_| {
                    *selected_for_select.borrow_mut() = Some(account_id.clone());
                    let selected_id = account_id.clone();
                    let status = status_for_select.clone();
                    let rt = rt_for_select.clone();
                    glib::MainContext::default().spawn_local(async move {
                        match rt
                            .spawn(async move {
                                save_selected_account(&selected_account_path(), Some(&selected_id)).await
                            })
                            .await
                        {
                            Ok(Ok(())) => status.set_text("Account selected."),
                            Ok(Err(error)) => status.set_text(&format!("Could not save selection: {error}")),
                            Err(error) => status.set_text(&format!("Could not save selection: {error}")),
                        }
                    });
                });

                let remove = Button::with_label("Remove");
                remove.add_css_class("destructive-action");
                let remove_id = account.id.clone();
                let accounts_for_remove = accounts.clone();
                let selected_for_remove = selected.clone();
                let status_for_remove = status.clone();
                let rt_for_remove = rt.clone();
                let rebuild_for_remove = rebuild_slot.borrow().clone();
                remove.connect_clicked(move |_| {
                    accounts_for_remove.borrow_mut().retain(|item| item.id != remove_id);
                    if selected_for_remove.borrow().as_deref() == Some(remove_id.as_str()) {
                        *selected_for_remove.borrow_mut() = accounts_for_remove
                            .borrow()
                            .first()
                            .map(|account| account.id.clone());
                    }

                    let remaining = accounts_for_remove.borrow().clone();
                    let selected_id = selected_for_remove.borrow().clone();
                    let status = status_for_remove.clone();
                    let rebuild = rebuild_for_remove.clone();
                    glib::MainContext::default().spawn_local(async move {
                        let result = rt_for_remove
                            .spawn(async move {
                                save_accounts(&accounts_path(), &remaining).await?;
                                save_selected_account(&selected_account_path(), selected_id.as_deref()).await
                            })
                            .await;
                        match result {
                            Ok(Ok(())) => {
                                status.set_text("Account removed.");
                                if let Some(rebuild) = rebuild {
                                    rebuild();
                                }
                            }
                            Ok(Err(error)) => status.set_text(&format!("Could not remove account: {error}")),
                            Err(error) => status.set_text(&format!("Could not remove account: {error}")),
                        }
                    });
                });

                controls.append(&select);
                controls.append(&remove);
                card.append(&Separator::new(Orientation::Horizontal));
                card.append(&controls);
                account_area.append(&card);
            }
        }
    });

    *rebuild_slot.borrow_mut() = Some(rebuild.clone());

    {
        let accounts = accounts.clone();
        let selected = selected.clone();
        let status = status.clone();
        let rebuild = rebuild.clone();
        let rt = rt.clone();
        glib::MainContext::default().spawn_local(async move {
            let loaded = rt.spawn(async { load_accounts(&accounts_path()).await }).await;
            match loaded {
                Ok(Ok(saved)) => *accounts.borrow_mut() = saved,
                Ok(Err(error)) => status.set_text(&format!("Could not load accounts: {error}")),
                Err(error) => status.set_text(&format!("Could not load accounts: {error}")),
            }

            let loaded_selected = rt
                .spawn(async { load_selected_account(&selected_account_path()).await })
                .await;
            if let Ok(Ok(id)) = loaded_selected {
                *selected.borrow_mut() = id;
            }

            if selected.borrow().is_none() {
                if let Some(first) = accounts.borrow().first() {
                    *selected.borrow_mut() = Some(first.id.clone());
                }
            }
            rebuild();
        });
    }

    {
        let accounts = accounts.clone();
        let selected = selected.clone();
        let status = status.clone();
        let rebuild = rebuild.clone();
        let rt = rt.clone();
        add_offline.connect_clicked(move |_| {
            let prompt = gtk::Window::builder()
                .title("Add Offline Account")
                .default_width(360)
                .default_height(180)
                .modal(true)
                .build();
            let prompt_root = GtkBox::new(Orientation::Vertical, 12);
            prompt_root.set_margin_top(20);
            prompt_root.set_margin_bottom(20);
            prompt_root.set_margin_start(20);
            prompt_root.set_margin_end(20);
            let entry = Entry::new();
            entry.set_placeholder_text(Some("Offline player name"));
            prompt_root.append(&entry);
            let add = Button::with_label("Add Account");
            add.add_css_class("suggested-action");
            prompt_root.append(&add);
            prompt.set_child(Some(&prompt_root));

            let prompt_for_add = prompt.clone();
            let accounts = accounts.clone();
            let selected = selected.clone();
            let status = status.clone();
            let rebuild = rebuild.clone();
            let rt = rt.clone();
            add.connect_clicked(move |_| {
                let name = entry.text().trim().to_string();
                if name.is_empty() {
                    status.set_text("Enter an offline player name.");
                    return;
                }

                let account = offline_account(name);
                let id = account.id.clone();
                accounts.borrow_mut().retain(|item| item.id != id);
                accounts.borrow_mut().push(account);
                *selected.borrow_mut() = Some(id.clone());

                let saved = accounts.borrow().clone();
                let status = status.clone();
                let rebuild = rebuild.clone();
                glib::MainContext::default().spawn_local(async move {
                    let result = rt
                        .spawn(async move {
                            save_accounts(&accounts_path(), &saved).await?;
                            save_selected_account(&selected_account_path(), Some(&id)).await
                        })
                        .await;
                    match result {
                        Ok(Ok(())) => {
                            status.set_text("Offline account added and selected.");
                            rebuild();
                        }
                        Ok(Err(error)) => status.set_text(&format!("Could not save account: {error}")),
                        Err(error) => status.set_text(&format!("Could not save account: {error}")),
                    }
                });
                prompt_for_add.close();
            });
            prompt.present();
        });
    }

    {
        let status = status.clone();
        let button = add_microsoft.clone();
        let device_area = device_area.clone();
        let device_code = device_code.clone();
        let open_signin = open_signin.clone();
        let accounts = accounts.clone();
        let selected = selected.clone();
        let rebuild = rebuild.clone();
        let rt = rt.clone();

        add_microsoft.connect_clicked(move |_| {
            button.set_sensitive(false);
            status.set_text("Requesting Microsoft sign-in code…");
            let status = status.clone();
            let button = button.clone();
            let device_area = device_area.clone();
            let device_code_entry = device_code.clone();
            let open_signin = open_signin.clone();
            let accounts = accounts.clone();
            let selected = selected.clone();
            let rebuild = rebuild.clone();
            let rt = rt.clone();

            glib::MainContext::default().spawn_local(async move {
                match rt.spawn(request_device_code()).await {
                    Ok(Ok(device)) => {
                        device_code_entry.set_text(&device.user_code);
                        device_area.set_visible(true);
                        status.set_text(&format!(
                            "Enter the code above on Microsoft. It expires in about {} minutes.",
                            (device.expires_in + 59) / 60
                        ));

                        let uri = device.verification_uri.clone();
                        open_signin.connect_clicked(move |_| {
                            let _ = gtk::gio::AppInfo::launch_default_for_uri(
                                &uri,
                                None::<&gtk::gio::AppLaunchContext>,
                            );
                        });

                        let status = status.clone();
                        let button = button.clone();
                        let device_area = device_area.clone();
                        let accounts = accounts.clone();
                        let selected = selected.clone();
                        let rebuild = rebuild.clone();
                        let rt = rt.clone();
                        glib::MainContext::default().spawn_local(async move {
                            match rt.spawn(complete_device_login(device)).await {
                                Ok(Ok(account)) => {
                                    let account_id = account.id.clone();
                                    accounts.borrow_mut().retain(|item| item.id != account_id);
                                    accounts.borrow_mut().push(account.clone());
                                    *selected.borrow_mut() = Some(account_id.clone());
                                    let saved = accounts.borrow().clone();
                                    let result = rt
                                        .spawn(async move {
                                            save_accounts(&accounts_path(), &saved).await?;
                                            save_selected_account(
                                                &selected_account_path(),
                                                Some(&account_id),
                                            )
                                            .await
                                        })
                                        .await;
                                    match result {
                                        Ok(Ok(())) => {
                                            status.set_text(&format!(
                                                "Signed in as {}. This account is selected.",
                                                account.name
                                            ));
                                            button.set_sensitive(true);
                                            device_area.set_visible(false);
                                            rebuild();
                                        }
                                        Ok(Err(error)) => status.set_text(&format!(
                                            "Signed in, but could not save the account: {error}"
                                        )),
                                        Err(error) => status.set_text(&format!(
                                            "Signed in, but could not save the account: {error}"
                                        )),
                                    }
                                }
                                Ok(Err(error)) => {
                                    status.set_text(&format!("Microsoft login failed: {error}"));
                                    button.set_sensitive(true);
                                    device_area.set_visible(false);
                                }
                                Err(error) => {
                                    status.set_text(&format!("Microsoft login failed: {error}"));
                                    button.set_sensitive(true);
                                    device_area.set_visible(false);
                                }
                            }
                        });
                    }
                    Ok(Err(error)) => {
                        status.set_text(&format!("Microsoft login failed: {error}"));
                        button.set_sensitive(true);
                    }
                    Err(error) => {
                        status.set_text(&format!("Microsoft login failed: {error}"));
                        button.set_sensitive(true);
                    }
                }
            });
        });
    }

    {
        let device_code = device_code.clone();
        copy_code.connect_clicked(move |_| {
            if let Some(display) = gtk::gdk::Display::default() {
                display.clipboard().set_text(&device_code.text());
            }
        });
    }

    let scrolled = gtk::ScrolledWindow::new();
    scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scrolled.set_vexpand(true);
    scrolled.set_child(Some(&root));
    scrolled
}
