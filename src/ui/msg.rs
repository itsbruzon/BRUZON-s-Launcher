use crate::models::{MinecraftVersion, Profile, Section, Theme};
use crate::settings::Settings;
use std::collections::HashMap;

#[derive(Debug)]
pub enum AppMsg {
    LaunchProfile(String),
    DeleteProfile(String),
    UsernameChanged(String),
    VersionSelected(String),
    RamChanged(u32),
    ToggleFabric(bool),
    SaveProfile,
    VersionsLoaded(Result<Vec<MinecraftVersion>, String>),
    ProfilesLoaded(Result<HashMap<String, Profile>, String>),
    DownloadProgress(f64, String),
    GameStarted,
    LaunchCompleted,
    NavigateToSection(Section),
    BackToMainMenu,
    OpenMinecraftFolder,
    ThemeSelected(Theme),
    ToggleHideLogs(bool),
    ToggleDiscordPresence(bool),
    ToggleSidebar,
    Log(String),
    Error(String),
    RequestDeleteProfile(String),
    SettingsLoaded(Settings),
    SessionEnded(String, u64),
}
