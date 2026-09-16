use adw::{self, NavigationPage, NavigationSplitView};
use relm4::gtk;

pub struct AppWidgets {
    pub window: adw::ApplicationWindow,
    pub header_bar: adw::HeaderBar,
    pub navigation_split_view: NavigationSplitView,
    pub navigation_page: NavigationPage,
    pub content_stack: gtk::Stack,

    // Pages
    pub home_page: gtk::Box,
    pub create_page: gtk::Box,
    pub settings_page: gtk::ScrolledWindow,
    pub logs_page: gtk::Box,
    pub loading_page: adw::StatusPage,

    // Home / create page widgets
    pub profile_list: gtk::ListBox,
    pub username_entry: adw::EntryRow,
    pub version_combo: adw::ComboRow,
    pub ram_scale: adw::SpinRow,
    pub fabric_switch: adw::SwitchRow,
    pub hide_logs_switch: adw::SwitchRow,

    // Sidebar buttons
    pub home_button: gtk::Button,
    pub create_sidebar_button: gtk::Button,
    pub settings_button: gtk::Button,
    pub logs_button: gtk::Button,

    // Sidebar button labels
    pub home_label: gtk::Label,
    pub create_label: gtk::Label,
    pub settings_label: gtk::Label,
    pub logs_label: gtk::Label,

    // Sidebar button boxes
    pub home_box: gtk::Box,
    pub create_box: gtk::Box,
    pub settings_box: gtk::Box,
    pub logs_box: gtk::Box,

    // Sidebar toggle
    pub sidebar_toggle_button: gtk::Button,

    // Settings
    pub theme_combo: adw::ComboRow,

    // Error / loading state
    pub error_label: gtk::Label,
    pub loading_spinner: gtk::Spinner,
    pub loading_progress: gtk::ProgressBar,
    pub loading_label: gtk::Label,

    // Toasts / logs
    pub toast_overlay: adw::ToastOverlay,
    pub logs_view: gtk::TextView,
}
