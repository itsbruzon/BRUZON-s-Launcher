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

    // Profiles is the account manager. Keep it visually consistent with the
    // other sidebar entries when the sidebar switches between expanded and
    // compact widths.
    let (profiles_button, profiles_label, profiles_box) =
        create_nav_button("Profiles", "avatar-default-symbolic");
    profiles_button.set_tooltip_text(Some("Manage Microsoft and offline accounts"));
    let profiles_sender = sender.clone();
    profiles_button.connect_clicked(move |_| {
        profiles_sender.input(AppMsg::NavigateToSection(Section::Profiles));
    });

    // Do not poll the sidebar width. A 100 ms timer was fighting the main
    // component's collapsed state while NavigationSplitView was resizing,
    // which caused the labels to flicker/jump during every toggle.
    // Instead, react to GTK's width notifications so Profiles follows the
    // same actual allocation without introducing a second resize loop.
    let profiles_label_for_resize = profiles_label.clone();
    let profiles_box_for_resize = profiles_box.clone();
    sidebar_content.connect_notify_local(Some("width"), move |sidebar, _| {
        let collapsed = sidebar.width() <= 100;
        profiles_label_for_resize.set_visible(!collapsed);
        profiles_box_for_resize.set_halign(if collapsed {
            gtk::Align::Center
        } else {
            gtk::Align::Start
        });
    });

    let profiles_label_for_idle = profiles_label.clone();
    let profiles_box_for_idle = profiles_box.clone();
    let sidebar_for_idle = sidebar_content.clone();
    gtk::glib::idle_add_local_once(move || {
        let collapsed = sidebar_for_idle.width() <= 100;
        profiles_label_for_idle.set_visible(!collapsed);
        profiles_box_for_idle.set_halign(if collapsed {
            gtk::Align::Center
        } else {
            gtk::Align::Start
        });
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
