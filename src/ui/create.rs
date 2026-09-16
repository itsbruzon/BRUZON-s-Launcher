use crate::ui::model::AppModel;
use crate::ui::msg::AppMsg;
use adw::prelude::*;
use adw::{ComboRow, EntryRow, SpinRow};
use relm4::ComponentSender;
use relm4::gtk;

pub fn create_create_instance_page(
    sender: &ComponentSender<AppModel>,
    username_entry: &EntryRow,
    version_combo: &ComboRow,
    ram_scale: &SpinRow,
    fabric_switch: &adw::SwitchRow,
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
        .label("New Instance")
        .halign(gtk::Align::Start)
        .css_classes(vec!["title-1".to_string()])
        .build();
    content_container.append(&title_label);

    let description = gtk::Label::builder()
        .label("Create an independent Minecraft installation. The selected account is chosen separately in Accounts.")
        .halign(gtk::Align::Start)
        .wrap(true)
        .css_classes(vec!["dim-label".to_string()])
        .build();
    content_container.append(&description);

    let input_list = gtk::ListBox::new();
    input_list.add_css_class("boxed-list");
    input_list.set_selection_mode(gtk::SelectionMode::None);
    input_list.set_hexpand(true);
    input_list.set_halign(gtk::Align::Fill);

    let sender_clone = sender.clone();
    username_entry.connect_changed(move |entry: &adw::EntryRow| {
        sender_clone.input(AppMsg::UsernameChanged(entry.text().to_string()));
    });
    username_entry.set_title("Instance Name");
    username_entry.set_text("");
    username_entry.set_hexpand(true);

    let sender_clone = sender.clone();
    version_combo.connect_notify(Some("selected"), move |combo: &adw::ComboRow, _| {
        if let Some(item) = combo.selected_item() {
            if let Some(string_obj) = item.downcast_ref::<gtk::StringObject>() {
                sender_clone.input(AppMsg::VersionSelected(string_obj.string().to_string()));
            }
        }
    });
    version_combo.set_hexpand(true);

    let sender_clone = sender.clone();
    ram_scale.adjustment().connect_value_changed(move |adj| {
        sender_clone.input(AppMsg::RamChanged(adj.value() as u32));
    });
    ram_scale.set_hexpand(true);

    let sender_clone = sender.clone();
    fabric_switch.connect_active_notify(move |switch| {
        sender_clone.input(AppMsg::ToggleFabric(switch.is_active()));
    });
    fabric_switch.set_hexpand(true);

    input_list.append(username_entry);
    input_list.append(version_combo);
    input_list.append(ram_scale);
    input_list.append(fabric_switch);
    content_container.append(&input_list);

    let button_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .halign(gtk::Align::Fill)
        .margin_top(12)
        .build();

    let save_button = gtk::Button::builder()
        .label("Create Instance")
        .css_classes(vec!["suggested-action".to_string()])
        .height_request(40)
        .hexpand(true)
        .build();
    let sender_clone = sender.clone();
    save_button.connect_clicked(move |_| sender_clone.input(AppMsg::SaveProfile));
    button_box.append(&save_button);
    content_container.append(&button_box);

    main_box.append(&content_container);
    main_box
}
