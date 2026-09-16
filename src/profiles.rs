use crate::auth::{complete_device_login, load_accounts, request_device_code, save_accounts, DeviceCode, MinecraftAccount};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Entry, Label, Orientation, Separator};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use tokio::runtime::Runtime;

fn accounts_path() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("BLauncher").join("accounts.json")
}

pub fn open_profiles_dialog(parent: Option<&gtk4::Window>) {
    let dialog = gtk4::Window::builder().title("Microsoft Accounts").default_width(430).default_height(520).modal(true).build();
    if let Some(parent) = parent { dialog.set_transient_for(Some(parent)); }

    let root = GtkBox::new(Orientation::Vertical, 18);
    root.set_margin_top(24); root.set_margin_bottom(24); root.set_margin_start(24); root.set_margin_end(24);
    let title = Label::new(Some("Accounts")); title.add_css_class("title-2"); root.append(&title);
    let account_area = GtkBox::new(Orientation::Vertical, 12); account_area.set_vexpand(true); account_area.set_valign(Align::Start); root.append(&account_area);
    let status = Label::new(None); status.set_wrap(true); status.set_selectable(true); root.append(&status);
    let sign_in = Button::with_label("Sign in with Microsoft"); sign_in.add_css_class("suggested-action"); root.append(&sign_in);
    let close = Button::with_label("Close"); root.append(&close);
    dialog.set_child(Some(&root));

    let rt = match Runtime::new() {
        Ok(rt) => Rc::new(rt),
        Err(error) => { status.set_text(&format!("Could not start authentication runtime: {error}")); dialog.present(); return; }
    };
    let path = accounts_path();
    let accounts: Rc<RefCell<Vec<MinecraftAccount>>> = Rc::new(RefCell::new(Vec::new()));

    let rebuild: Rc<dyn Fn()> = Rc::new({
        let account_area = account_area.clone(); let accounts = accounts.clone(); let path = path.clone(); let rt = rt.clone(); let status = status.clone();
        move || {
            while let Some(child) = account_area.first_child() { account_area.remove(&child); }
            for account in accounts.borrow().clone() {
                let card = GtkBox::new(Orientation::Vertical, 10); card.set_halign(Align::Center); card.set_hexpand(true); card.add_css_class("card");
                let avatar = Label::new(Some("👤")); avatar.set_size_request(72, 72); avatar.set_halign(Align::Center); avatar.add_css_class("title-1"); card.append(&avatar);
                let name = Label::new(Some(&account.name)); name.add_css_class("title-2"); name.set_halign(Align::Center); card.append(&name);
                let uuid = Label::new(Some(&format!("UUID: {}", account.id))); uuid.set_selectable(true); uuid.set_halign(Align::Center); uuid.add_css_class("dim-label"); card.append(&uuid);
                if account.skin_url.is_some() { let hint = Label::new(Some("Minecraft profile loaded")); hint.add_css_class("dim-label"); hint.set_halign(Align::Center); card.append(&hint); }
                let separator = Separator::new(Orientation::Horizontal); separator.set_margin_top(6); separator.set_margin_bottom(6); card.append(&separator);
                let sign_out = Button::with_label("Sign Out"); sign_out.add_css_class("destructive-action"); sign_out.set_halign(Align::Center);
                let accounts_for_signout = accounts.clone(); let path_for_signout = path.clone(); let rt_for_signout = rt.clone(); let account_id = account.id.clone(); let status_for_signout = status.clone(); let account_for_restore = account.clone(); let account_area_for_signout = account_area.clone(); let card_for_signout = card.clone();
                sign_out.connect_clicked(move |_| {
                    accounts_for_signout.borrow_mut().retain(|saved| saved.id != account_id);
                    let remaining = accounts_for_signout.borrow().clone(); let path = path_for_signout.clone(); let status = status_for_signout.clone(); let rt = rt_for_signout.clone(); let accounts = accounts_for_signout.clone(); let account = account_for_restore.clone(); let account_area = account_area_for_signout.clone(); let card = card_for_signout.clone();
                    glib::MainContext::default().spawn_local(async move {
                        match rt.spawn(async move { save_accounts(&path, &remaining).await }).await {
                            Ok(Ok(())) => { account_area.remove(&card); status.set_text("Account signed out."); }
                            Ok(Err(error)) => { accounts.borrow_mut().push(account); status.set_text(&format!("Could not save accounts: {error}")); }
                            Err(error) => { accounts.borrow_mut().push(account); status.set_text(&format!("Could not save accounts: {error}")); }
                        }
                    });
                });
                card.append(&sign_out); account_area.append(&card);
            }
        }
    });

    {
        let accounts = accounts.clone(); let path = path.clone(); let status = status.clone(); let rebuild = rebuild.clone(); let rt = rt.clone();
        glib::MainContext::default().spawn_local(async move {
            match rt.spawn(async move { load_accounts(&path).await }).await {
                Ok(Ok(saved)) => { *accounts.borrow_mut() = saved; rebuild(); }
                Ok(Err(error)) => status.set_text(&format!("Could not load accounts: {error}")),
                Err(error) => status.set_text(&format!("Could not load accounts: {error}")),
            }
        });
    }

    {
        let status = status.clone(); let sign_in = sign_in.clone(); let rt = rt.clone(); let accounts = accounts.clone(); let path = path.clone(); let rebuild = rebuild.clone();
        sign_in.clone().connect_clicked(move |_| {
            sign_in.set_sensitive(false); status.set_text("Requesting Microsoft sign-in code…");
            let status_for_device = status.clone(); let sign_in_for_device = sign_in.clone(); let accounts_for_device = accounts.clone(); let path_for_device = path.clone(); let rt_for_device = rt.clone(); let rebuild_for_device = rebuild.clone();
            glib::MainContext::default().spawn_local(async move {
                let device_result = rt_for_device.spawn(request_device_code()).await;
                let device = match device_result {
                    Ok(Ok(device)) => device,
                    Ok(Err(error)) => { status_for_device.set_text(&format!("Microsoft login failed: {error}")); sign_in_for_device.set_sensitive(true); return; }
                    Err(error) => { status_for_device.set_text(&format!("Microsoft login failed: {error}")); sign_in_for_device.set_sensitive(true); return; }
                };
                show_device_code(&status_for_device, &device, &sign_in_for_device, accounts_for_device, path_for_device, rt_for_device, rebuild_for_device);
            });
        });
    }

    let dialog_for_close = dialog.clone(); close.connect_clicked(move |_| dialog_for_close.close());
    dialog.present();
}

fn show_device_code(status: &Label, device: &DeviceCode, sign_in: &Button, accounts: Rc<RefCell<Vec<MinecraftAccount>>>, path: PathBuf, rt: Rc<Runtime>, rebuild: Rc<dyn Fn()>) {
    status.set_text("Enter this code on the Microsoft sign-in page:");
    let parent = status.root().and_downcast::<gtk4::Window>();
    let dialog = gtk4::Window::builder().title("Microsoft Sign-in").default_width(390).default_height(280).modal(true).build();
    if let Some(parent) = parent { dialog.set_transient_for(Some(&parent)); }
    let box_root = GtkBox::new(Orientation::Vertical, 14); box_root.set_margin_top(24); box_root.set_margin_bottom(24); box_root.set_margin_start(24); box_root.set_margin_end(24);
    let code = Entry::new(); code.set_text(&device.user_code); code.set_editable(false); code.set_can_focus(true); code.set_halign(Align::Center); code.add_css_class("title-2"); box_root.append(&code);
    let copy = Button::with_label("Copy code"); box_root.append(&copy);
    let open = Button::with_label("Open sign-in page"); open.add_css_class("suggested-action"); box_root.append(&open);
    let info = Label::new(Some(&format!("Code expires in about {} minutes.", (device.expires_in + 59) / 60))); info.add_css_class("dim-label"); info.set_wrap(true); box_root.append(&info);
    let done = Button::with_label("Waiting for sign-in…"); done.set_sensitive(false); box_root.append(&done);
    dialog.set_child(Some(&box_root));
    { let code = code.clone(); copy.connect_clicked(move |_| { if let Some(display) = gtk4::gdk::Display::default() { display.clipboard().set_text(&code.text()); } }); }
    { let uri = device.verification_uri.clone(); open.connect_clicked(move |_| { let _ = gtk4::gio::AppInfo::launch_default_for_uri(&uri, None::<&gtk4::gio::AppLaunchContext>); }); }
    {
        let status = status.clone(); let sign_in = sign_in.clone(); let accounts = accounts.clone(); let path = path.clone(); let rt = rt.clone(); let dialog = dialog.clone(); let rebuild = rebuild.clone(); let device = device.clone();
        glib::MainContext::default().spawn_local(async move {
            match rt.spawn(complete_device_login(device)).await {
                Ok(Ok(account)) => {
                    accounts.borrow_mut().retain(|existing| existing.id != account.id); accounts.borrow_mut().push(account.clone()); let saved = accounts.borrow().clone();
                    match rt.spawn(async move { save_accounts(&path, &saved).await }).await {
                        Ok(Ok(())) => { status.set_text(&format!("Signed in as {}", account.name)); sign_in.set_sensitive(true); rebuild(); dialog.close(); }
                        Ok(Err(error)) => status.set_text(&format!("Signed in, but could not save account: {error}")),
                        Err(error) => status.set_text(&format!("Signed in, but could not save account: {error}")),
                    }
                }
                Ok(Err(error)) => { status.set_text(&format!("Microsoft login failed: {error}")); sign_in.set_sensitive(true); dialog.close(); }
                Err(error) => { status.set_text(&format!("Microsoft login failed: {error}")); sign_in.set_sensitive(true); dialog.close(); }
            }
        });
    }
    dialog.present();
}
