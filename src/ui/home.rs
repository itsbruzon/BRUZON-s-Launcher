use crate::auth::selected_account_path;
use crate::models::Profile;
use crate::ui::model::AppModel;
use crate::ui::msg::AppMsg;
use gtk::prelude::*;
use relm4::ComponentSender;
use relm4::gtk;

pub fn create_home_page(
    _sender: &ComponentSender<AppModel>,
    profile_list: &gtk::ListBox,
) -> gtk::Box {
    let main_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .hexpand(true)
        .vexpand(true)
        .build();

    let content_container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();

    let title_label = gtk::Label::builder()
        .label("Instances")
        .halign(gtk::Align::Start)
        .css_classes(vec!["title-1".to_string()])
        .build();
    content_container.append(&title_label);

    let subtitle = gtk::Label::builder()
        .label("Each instance has its own Minecraft version, loader, files, mods and resources. The selected account is used when you launch it.")
        .halign(gtk::Align::Start)
        .wrap(true)
        .css_classes(vec!["dim-label".to_string()])
        .build();
    content_container.append(&subtitle);

    profile_list.set_selection_mode(gtk::SelectionMode::None);
    profile_list.add_css_class("boxed-list");
    content_container.append(profile_list);

    main_box.append(&content_container);
    main_box
}

pub fn update_profile_list(
    profile_list: &gtk::ListBox,
    profiles: &std::collections::HashMap<String, Profile>,
    sender: &ComponentSender<AppModel>,
) {
    while let Some(child) = profile_list.first_child() {
        profile_list.remove(&child);
    }

    if profiles.is_empty() {
        let label = gtk::Label::builder()
            .label("No instances yet. Create one to get started!")
            .halign(gtk::Align::Center)
            .margin_top(24)
            .margin_bottom(24)
            .build();
        profile_list.append(&label);
    } else {
        for (name, profile) in profiles {
            profile_list.append(&create_profile_row(name, profile, sender));
        }
    }
}

fn create_profile_row(
    name: &str,
    profile: &Profile,
    sender: &ComponentSender<AppModel>,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let box_container = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    let info_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
        .halign(gtk::Align::Start)
        .build();

    let name_label = gtk::Label::builder()
        .label(name)
        .halign(gtk::Align::Start)
        .css_classes(vec!["title-4".to_string()])
        .build();

    let hours = profile.playtime_seconds / 3600;
    let minutes = (profile.playtime_seconds % 3600) / 60;
    let playtime = if hours > 0 { format!("{}h {}m", hours, minutes) } else if minutes > 0 { format!("{}m", minutes) } else { "No playtime".to_string() };
    let version = if profile.is_fabric { format!("{} • Fabric", profile.version) } else { profile.version.clone() };
    let details = gtk::Label::builder()
        .label(format!("{} • {} MB • {}", version, profile.ram_mb, playtime))
        .halign(gtk::Align::Start)
        .css_classes(vec!["dim-label".to_string()])
        .build();

    info_box.append(&name_label);
    info_box.append(&details);

    let button_box = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(6).build();
    let launch_button = gtk::Button::builder()
        .label("Launch")
        .css_classes(vec!["suggested-action".to_string()])
        .valign(gtk::Align::Center)
        .build();
    let sender_clone = sender.clone();
    let name_clone = name.to_string();
    launch_button.connect_clicked(move |_| {
        let has_selected_account = std::fs::read_to_string(selected_account_path())
            .map(|id| !id.trim().is_empty())
            .unwrap_or(false);

        if has_selected_account {
            sender_clone.input(AppMsg::LaunchProfile(name_clone.clone()));
        } else {
            sender_clone.input(AppMsg::Error(
                "You need to add and select an account before you can launch Minecraft."
                    .to_string(),
            ));
        }
    });

    let delete_button = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text("Delete instance")
        .css_classes(vec!["destructive-action".to_string()])
        .valign(gtk::Align::Center)
        .build();
    let sender_clone = sender.clone();
    let name_clone = name.to_string();
    delete_button.connect_clicked(move |_| sender_clone.input(AppMsg::RequestDeleteProfile(name_clone.clone())));

    button_box.append(&launch_button);
    button_box.append(&delete_button);
    box_container.append(&info_box);
    box_container.append(&button_box);
    row.set_child(Some(&box_container));
    row.set_activatable(false);
    row
}
