use crate::models::Section;
use crate::ui::model::AppModel;
use crate::ui::msg::AppMsg;
use adw::NavigationPage;
use adw::prelude::*;
use relm4::ComponentSender;
use relm4::gtk;

pub fn create_sidebar(
    sender: &ComponentSender<AppModel>,
) -> (
    NavigationPage,
    gtk::Button,
    gtk::Button,
    gtk::Button,
    gtk::Button,
    gtk::Label,
    gtk::Label,
    gtk::Label,
    gtk::Label,
    gtk::Box,
    gtk::Box,
    gtk::Box,
    gtk::Box,
) {
    let sidebar_content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .vexpand(true)
        .hexpand(true)
        .halign(gtk::Align::Fill)
        .valign(gtk::Align::Fill)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let create_nav_button = |label_text: &str, icon_name: &str| -> (gtk::Button, gtk::Label, gtk::Box) {
        let button = gtk::Button::builder()
            .halign(gtk::Align::Fill)
            .hexpand(true)
            .height_request(40)
            .margin_top(6)
            .margin_bottom(6)
            .build();
        let box_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .halign(gtk::Align::Start)
            .build();
        let icon = gtk::Image::builder().icon_name(icon_name).build();
        let label = gtk::Label::builder().label(label_text).visible(true).build();
        box_container.append(&icon);
        box_container.append(&label);
        button.set_child(Some(&box_container));
        (button, label, box_container)
    };

    let (home_button, home_label, home_box) = create_nav_button("Home", "user-home-symbolic");
    let (create_button, create_label, create_box) = create_nav_button("New Instance", "list-add-symbolic");
    let (settings_button, settings_label, settings_box) = create_nav_button("Settings", "emblem-system-symbolic");
    let (logs_button, logs_label, logs_box) = create_nav_button("Logs", "utilities-terminal-symbolic");
    logs_button.set_visible(false);

    // Profiles is the account manager. Accounts are deliberately kept separate
    // from instances: an instance never stores which account launched it.
    let (profiles_button, profiles_label, profiles_box) =
        create_nav_button("Profiles", "avatar-default-symbolic");
    profiles_button.set_tooltip_text(Some("Manage Microsoft and offline accounts"));
    let profiles_sender = sender.clone();
    profiles_button.connect_clicked(move |_| {
        profiles_sender.input(AppMsg::NavigateToSection(Section::Profiles));
    });

    // Keep every sidebar label in sync with the actual sidebar width, including
    // the Profiles button at the bottom. The idle pass handles the initial
    // allocation because width_notify is not guaranteed to fire on startup.
    let labels = [
        home_label.clone(),
        create_label.clone(),
        settings_label.clone(),
        logs_label.clone(),
        profiles_label.clone(),
    ];
    let boxes = [
        home_box.clone(),
        create_box.clone(),
        settings_box.clone(),
        logs_box.clone(),
        profiles_box.clone(),
    ];
    let sync_sidebar = move |width: i32| {
        let collapsed = width <= 100;
        for label in &labels {
            label.set_visible(!collapsed);
        }
        for box_container in &boxes {
            box_container.set_halign(if collapsed {
                gtk::Align::Center
            } else {
                gtk::Align::Start
            });
        }
    };

    let sync_on_resize = sync_sidebar.clone();
    sidebar_content.connect_width_notify(move |sidebar| {
        sync_on_resize(sidebar.width());
    });

    let sync_initial = sync_sidebar.clone();
    glib::idle_add_local_once(move || {
        sync_initial(sidebar_content.width());
    });

    let sender_clone = sender.clone();
    home_button.connect_clicked(move |_| sender_clone.input(AppMsg::NavigateToSection(Section::Home)));
    let sender_clone = sender.clone();
    create_button.connect_clicked(move |_| sender_clone.input(AppMsg::NavigateToSection(Section::CreateInstance)));
    let sender_clone = sender.clone();
    settings_button.connect_clicked(move |_| sender_clone.input(AppMsg::NavigateToSection(Section::Settings)));
    let sender_clone = sender.clone();
    logs_button.connect_clicked(move |_| sender_clone.input(AppMsg::NavigateToSection(Section::Logs)));

    sidebar_content.append(&home_button);
    sidebar_content.append(&create_button);
    sidebar_content.append(&logs_button);
    sidebar_content.append(&settings_button);

    let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    sidebar_content.append(&spacer);
    sidebar_content.append(&profiles_button);

    let version_label = gtk::Label::builder()
        .label("v0.1-dev")
        .css_classes(vec!["dim-label".to_string(), "subtitle".to_string()])
        .margin_bottom(12)
        .build();
    sidebar_content.append(&version_label);

    let sidebar_page = adw::NavigationPage::builder()
        .title("Navigation")
        .child(&sidebar_content)
        .vexpand(true)
        .hexpand(true)
        .build();
    sidebar_page.set_css_classes(&["flat"]);

    (
        sidebar_page,
        home_button,
        create_button,
        settings_button,
        logs_button,
        home_label,
        create_label,
        settings_label,
        logs_label,
        home_box,
        create_box,
        settings_box,
        logs_box,
    )
}
