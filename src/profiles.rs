use crate::auth::{complete_device_login, load_accounts, offline_account, request_device_code, save_accounts, save_selected_account, AccountType, DeviceCode, MinecraftAccount};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Entry, Label, Orientation, Separator};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use tokio::runtime::Runtime;

fn accounts_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("BLauncher")
}

fn accounts_path() -> PathBuf {
    accounts_dir().join("accounts.json")
}

fn selected_account_path() -> PathBuf {
    accounts_dir().join("selected_account")
}

pub fn open_profiles_dialog(parent: Option<&gtk4::Window>) {
    let dialog = gtk4::Window::builder()
        .title("Accounts")
        .default_width(460)
        .default_height(620)
        .modal(true)
        .build();
    if let Some(parent) = parent {
        dialog.set_transient_for(Some(parent));
    }

    let root = GtkBox::new(Orientation::Vertical, 14);
    root.set_margin_top(20);
    root.set_margin_bottom(20);
    root.set_margin_start(20);
    root.set_margin_end(20);

    let title = Label::new(Some("Accounts"));
    title.add_css_class("title-1");
    title.set_halign(Align::Start);
    root.append(&title);

    let subtitle = Label::new(Some("The selected account is used when launching every instance."));
    subtitle.add_css_class("dim-label");
    subtitle.set_wrap(true);
    subtitle.set_halign(Align::Start);
    root.append(&subtitle);

    let account_area = GtkBox::new(Orientation::Vertical, 10);
    account_area.set_vexpand(true);
    account_area.set_valign(Align::Start);
    root.append(&account_area);

    let status = Label::new(None);
    status.set_wrap(true);
    status.set_selectable(true);
    root.append(&status);

    let actions = GtkBox::new(Orientation::Horizontal, 8);
    let add_offline = Button::with_label("Add Offline Account");
    let add_microsoft = Button::with_label("Sign in with Microsoft");
    add_microsoft.add_css_class("suggested-action");
    let close = Button::with_label("Close");
    actions.append(&add_offline);
    actions.append(&add_microsoft);
    actions.append(&close);
    root.append(&actions);
    dialog.set_child(Some(&root));

    let rt = match Runtime::new() {
        Ok(rt) => Rc::new(rt),
        Err(error) => {
            status.set_text(&format!("Could not start account runtime: {error}"));
            dialog.present();
            return;
        }
    };

    let accounts: Rc<RefCell<Vec<MinecraftAccount>>> = Rc::new(RefCell::new(Vec::new()));
    let selected: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let rebuild: Rc<dyn Fn()> = Rc::new({
        let account_area = account_area.clone();
        let accounts = accounts.clone();
        let selected = selected.clone();
        let status = status.clone();
        let rt = rt.clone();
        move || {
            while let Some(child) = account_area.first_child() {
                account_area.remove(&child);
            }

            for account in accounts.borrow().clone() {
                let card = GtkBox::new(Orientation::Vertical, 8);
                card.set_hexpand(true);
                card.add_css_class("card");

                let header = GtkBox::new(Orientation::Horizontal, 10);
                let avatar = Label::new(Some(if account.account_type == AccountType::Microsoft { "☁" } else { "👤" }));
                avatar.set_size_request(48, 48);
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
                let accounts_for_select = accounts.clone();
                let selected_for_select = selected.clone();
                let status_for_select = status.clone();
                let rt_for_select = rt.clone();
                select.connect_clicked(move |_| {
                    *selected_for_select.borrow_mut() = Some(account_id.clone());
                    let selected_id = account_id.clone();
                    let status = status_for_select.clone();
                    let rt = rt_for_select.clone();
                    let _ = accounts_for_select.borrow();
                    glib::MainContext::default().spawn_local(async move {
                        match rt.spawn(async move { save_selected_account(&selected_account_path(), Some(&selected_id)).await }).await {
                            Ok(Ok(())) => status.set_text("Account selected. Instances will use it on launch."),
                            Ok(Err(error)) => status.set_text(&format!("Could not save selected account: {error}")),
                            Err(error) => status.set_text(&format!("Could not save selected account: {error}")),
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
                remove.connect_clicked(move |_| {
                    accounts_for_remove.borrow_mut().retain(|item| item.id != remove_id);
                    if selected_for_remove.borrow().as_deref() == Some(remove_id.as_str()) {
                        *selected_for_remove.borrow_mut() = accounts_for_remove.borrow().first().map(|a| a.id.clone());
                    }
                    let remaining = accounts_for_remove.borrow().clone();
                    let selected_id = selected_for_remove.borrow().clone();
                    let status = status_for_remove.clone();
                    let rt = rt_for_remove.clone();
                    glib::MainContext::default().spawn_local(async move {
                        let save_result = rt.spawn(async move { save_accounts(&accounts_path(), &remaining).await }).await;
                        if let Err(error) = save_result {
                            status.set_text(&format!("Could not save accounts: {error}"));
                            return;
                        }
                        let save_result = rt.spawn(async move { save_selected_account(&selected_account_path(), selected_id.as_deref()).await }).await;
                        match save_result {
                            Ok(Ok(())) => status.set_text("Account removed."),
                            Ok(Err(error)) => status.set_text(&format!("Could not save selection: {error}")),
                            Err(error) => status.set_text(&format!("Could not save selection: {error}")),
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

    {
        let accounts = accounts.clone();
        let selected = selected.clone();
        let rebuild = rebuild.clone();
        let status = status.clone();
        let rt = rt.clone();
        glib::MainContext::default().spawn_local(async move {
            let loaded = rt.spawn(async move { load_accounts(&accounts_path()).await }).await;
            match loaded {
                Ok(Ok(saved)) => *accounts.borrow_mut() = saved,
                Ok(Err(error)) => status.set_text(&format!("Could not load accounts: {error}")),
                Err(error) => status.set_text(&format!("Could not load accounts: {error}")),
            }
            let loaded_selected = rt.spawn(async move { crate::auth::load_selected_account(&selected_account_path()).await }).await;
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
        add_offline.connect_clicked(move |_| {
            let prompt = gtk4::Window::builder().title("Add Offline Account").default_width(360).default_height(180).modal(true).build();
            let box_root = GtkBox::new(Orientation::Vertical, 12);
            box_root.set_margin_top(20); box_root.set_margin_bottom(20); box_root.set_margin_start(20); box_root.set_margin_end(20);
            let entry = Entry::new(); entry.set_placeholder_text(Some("Offline player name")); box_root.append(&entry);
            let add = Button::with_label("Add Account"); add.add_css_class("suggested-action"); box_root.append(&add);
            prompt.set_child(Some(&box_root));
            let accounts = accounts.clone(); let selected = selected.clone(); let status = status.clone(); let rebuild = rebuild.clone();
            add.connect_clicked(move |_| {
                let name = entry.text().trim().to_string();
                if name.is_empty() { status.set_text("Enter an offline player name."); return; }
                let account = offline_account(name);
                let id = account.id.clone();
                accounts.borrow_mut().retain(|item| item.id != id);
                accounts.borrow_mut().push(account);
                *selected.borrow_mut() = Some(id.clone());
                let saved = accounts.borrow().clone();
                let status = status.clone(); let rebuild = rebuild.clone();
                glib::MainContext::default().spawn_local(async move {
                    let save_accounts_result = tokio::fs::write(&accounts_path(), serde_json::to_vec_pretty(&saved).unwrap_or_default()).await;
                    if let Err(error) = save_accounts_result { status.set_text(&format!("Could not save account: {error}")); return; }
                    let _ = save_selected_account(&selected_account_path(), Some(&id)).await;
                    status.set_text("Offline account added and selected.");
                    rebuild();
                });
                prompt.close();
            });
            prompt.present();
        });
    }

    {
        let status = status.clone();
        let add_microsoft_clone = add_microsoft.clone();
        let accounts = accounts.clone();
        let selected = selected.clone();
        let rt = rt.clone();
        let rebuild = rebuild.clone();
        add_microsoft.connect_clicked(move |_| {
            add_microsoft_clone.set_sensitive(false);
            status.set_text("Requesting Microsoft sign-in code…");
            let status = status.clone(); let button = add_microsoft_clone.clone(); let accounts = accounts.clone(); let selected = selected.clone(); let rt = rt.clone(); let rebuild = rebuild.clone();
            glib::MainContext::default().spawn_local(async move {
                match rt.spawn(request_device_code()).await {
                    Ok(Ok(device)) => show_device_code(&status, &device, &button, accounts, selected, rt, rebuild),
                    Ok(Err(error)) => { status.set_text(&format!("Microsoft login failed: {error}")); button.set_sensitive(true); }
                    Err(error) => { status.set_text(&format!("Microsoft login failed: {error}")); button.set_sensitive(true); }
                }
            });
        });
    }

    let dialog_for_close = dialog.clone();
    close.connect_clicked(move |_| dialog_for_close.close());
    dialog.present();
}

fn show_device_code(status: &Label, device: &DeviceCode, button: &Button, accounts: Rc<RefCell<Vec<MinecraftAccount>>>, selected: Rc<RefCell<Option<String>>>, rt: Rc<Runtime>, rebuild: Rc<dyn Fn()>) {
    status.set_text("Enter this code on the Microsoft sign-in page:");
    let parent = status.root().and_downcast::<gtk4::Window>();
    let dialog = gtk4::Window::builder().title("Microsoft Sign-in").default_width(390).default_height(280).modal(true).build();
    if let Some(parent) = parent { dialog.set_transient_for(Some(&parent)); }
    let root = GtkBox::new(Orientation::Vertical, 14);
    root.set_margin_top(24); root.set_margin_bottom(24); root.set_margin_start(24); root.set_margin_end(24);
    let code = Entry::new(); code.set_text(&device.user_code); code.set_editable(false); code.set_halign(Align::Center); code.add_css_class("title-2"); root.append(&code);
    let copy = Button::with_label("Copy code"); root.append(&copy);
    let open = Button::with_label("Open sign-in page"); open.add_css_class("suggested-action"); root.append(&open);
    let info = Label::new(Some(&format!("Code expires in about {} minutes.", (device.expires_in + 59) / 60))); info.add_css_class("dim-label"); root.append(&info);
    dialog.set_child(Some(&root));
    { let code = code.clone(); copy.connect_clicked(move |_| { if let Some(display) = gtk4::gdk::Display::default() { display.clipboard().set_text(&code.text()); } }); }
    { let uri = device.verification_uri.clone(); open.connect_clicked(move |_| { let _ = gtk4::gio::AppInfo::launch_default_for_uri(&uri, None::<&gtk4::gio::AppLaunchContext>); }); }
    {
        let status = status.clone(); let button = button.clone(); let accounts = accounts.clone(); let selected = selected.clone(); let rt = rt.clone(); let dialog = dialog.clone(); let device = device.clone();
        glib::MainContext::default().spawn_local(async move {
            match rt.spawn(complete_device_login(device)).await {
                Ok(Ok(account)) => {
                    *selected.borrow_mut() = Some(account.id.clone());
                    accounts.borrow_mut().retain(|existing| existing.id != account.id);
                    accounts.borrow_mut().push(account.clone());
                    let saved = accounts.borrow().clone();
                    let selected_id = selected.borrow().clone();
                    let result = rt.spawn(async move {
                        save_accounts(&accounts_path(), &saved).await?;
                        save_selected_account(&selected_account_path(), selected_id.as_deref()).await
                    }).await;
                    match result {
                        Ok(Ok(())) => { status.set_text(&format!("Signed in as {} and selected.", account.name)); button.set_sensitive(true); rebuild(); dialog.close(); }
                        Ok(Err(error)) => status.set_text(&format!("Signed in, but could not save account: {error}")),
                        Err(error) => status.set_text(&format!("Signed in, but could not save account: {error}")),
                    }
                }
                Ok(Err(error)) => { status.set_text(&format!("Microsoft login failed: {error}")); button.set_sensitive(true); dialog.close(); }
                Err(error) => { status.set_text(&format!("Microsoft login failed: {error}")); button.set_sensitive(true); dialog.close(); }
            }
        });
    }
    dialog.present();
}
