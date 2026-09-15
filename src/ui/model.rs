use std::collections::HashMap;
use relm4::{ComponentSender, gtk};
use crate::models::{MinecraftVersion, Profile, Section};
use crate::settings::Settings;
use crate::launcher::MinecraftLauncher;

#[derive(Debug, Clone)]
pub enum AppState {
    Loading,
    Ready { current_section: Section },
    Downloading { version: String, progress: f64, status: String },
    Launching { version: String },
    GameRunning { #[allow(dead_code)] version: String },
    Error { message: String },
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Loading
    }
}

pub struct AppModel {
    pub state: AppState,
    pub launcher: Option<MinecraftLauncher>,
    pub window: Option<adw::ApplicationWindow>,

    // Data
    pub profiles: HashMap<String, Profile>,
    pub available_versions: Vec<MinecraftVersion>,
    pub sorted_versions: Vec<String>,

    // Inputs
    pub input_username: String,
    pub input_version: Option<String>,
    pub input_ram: u32,
    pub input_install_fabric: bool,
    pub fabric_switch_enabled: bool,

    // Settings & Logs
    pub settings: Settings,
    pub logs: gtk::TextBuffer,

    // UI State
    pub error_message: Option<String>,

    pub sidebar_collapsed: bool,

    pub versions_updated: bool,
    pub version_list_model: Option<gtk::StringList>,

    pub toast_overlay: Option<adw::ToastOverlay>,

    pub pending_launch_profile: Option<String>,

    // Component sender for UI updates
    pub sender: ComponentSender<AppModel>,

    pub java_dialog_request: Option<u32>,

    // Shared Tokio Runtime
    pub rt: std::sync::Arc<tokio::runtime::Runtime>,
}
