pub mod create;
pub mod home;
pub mod loading;
pub mod logs;
pub mod model;
pub mod msg;
pub mod profiles;
pub mod settings;
pub mod sidebar;
pub mod widgets;

pub use model::AppModel;
pub use msg::AppMsg;

use adw::prelude::*;
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};
use std::collections::HashMap;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::runtime::Runtime;

use crate::launcher::MinecraftLauncher;
use crate::minecraft_rpc::{MinecraftRpc, RpcPresence};
use crate::models::{Profile, Section, Theme};
use crate::settings::Settings;
use crate::ui::create::create_create_instance_page;
use crate::ui::home::{create_home_page, update_profile_list};
use crate::ui::loading::create_loading_widgets;
use crate::ui::logs::create_logs_page;
use crate::ui::model::AppState;
use crate::ui::profiles::create_profiles_page;
use crate::ui::settings::create_settings_page;
use crate::ui::sidebar::create_sidebar;
use crate::ui::widgets::AppWidgets;

impl SimpleComponent for AppModel {
    type Input = AppMsg;
    type Output = ();
    type Init = ();
    type Root = adw::ApplicationWindow;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        let window = adw::ApplicationWindow::builder()
            .title("BRUZON's Launcher")
            .default_width(900)
            .default_height(540)
            .build();
        window.set_decorated(true);

        // Load CSS for transparency
        let provider = gtk::CssProvider::new();
        provider.load_from_data("
            .transparent-window { background-color: rgba(30, 30, 30, 0.85); }
            .transparent-window navigation-split-view { background-color: transparent; }
            .transparent-window navigation-split-view > sidebar { background-color: transparent; border: none; }
            .transparent-window navigation-split-view > content { background-color: transparent; }
            .transparent-window .background { background-color: transparent; }
            .transparent-window .view { background-color: transparent; }
            .transparent-window .sidebar-pane { background-color: transparent; }
            
            /* Apply sidebar color (solid lighter gray) to content containers */
            .transparent-window list { background-color: #383838; }
            .transparent-window row { background-color: transparent; }
            
            /* Ensure sidebar buttons don't have opaque backgrounds unless active */
            .transparent-window .navigation-sidebar-item { background-color: transparent; }

            /* Semi-transparent lighter gray interactive elements (0.9 alpha) */
            .transparent-window button { background-color: alpha(#383838, 0.9); }
            .transparent-window entry { background-color: alpha(@theme_base_color, 0.9); }

            /* Active states */
            .transparent-window button.suggested-action { 
                background-color: @accent_bg_color; 
                color: @accent_fg_color;
            }
            .transparent-window button:checked {
                 background-color: @accent_bg_color;
                 color: @accent_fg_color;
            }
            
            /* Remove background from titlebar buttons */
            .transparent-window headerbar button { background-color: transparent; box-shadow: none; border: none; }
        ");

        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        window
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Initialize model
        let mut model = AppModel {
            state: AppState::Loading,
            launcher: match MinecraftLauncher::new() {
                Ok(launcher) => Some(launcher),
                Err(e) => {
                    sender.input(AppMsg::Error(e.to_string()));
                    None
                }
            },
            window: Some(root.clone()),
            profiles: HashMap::new(),
            available_versions: Vec::new(),
            sorted_versions: Vec::new(),
            input_username: String::new(),
            input_version: None,

            input_ram: 4096, // Default 4GB
            input_install_fabric: false,
            fabric_switch_enabled: false,
            error_message: None,
            sidebar_collapsed: false,

            // Initialize settings
            settings: Settings::default(), // Async load triggered later
            logs: gtk::TextBuffer::new(None),

            versions_updated: false,
            version_list_model: None,

            toast_overlay: None,
            sender: sender.clone(),
            rt: std::sync::Arc::new(Runtime::new().unwrap()),
            discord_rpc: MinecraftRpc::start(),
        };

        // Set window title
        root.set_title(Some("BRUZON's Launcher"));

        // Create navigation split view for sidebar navigation
        let navigation_split_view = adw::NavigationSplitView::new();
        navigation_split_view.set_collapsed(false);
        navigation_split_view.set_vexpand(true);
        navigation_split_view.set_hexpand(true);

        navigation_split_view.set_max_sidebar_width(250.0);
        navigation_split_view.set_min_sidebar_width(60.0);

        // Create sidebar
        let (
            sidebar,
            home_button,
            create_sidebar_button,
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
        ) = create_sidebar(&sender);
        navigation_split_view.set_sidebar(Some(&sidebar));

        // Create content stack for different sections
        let content_stack = gtk::Stack::new();
        content_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        content_stack.set_transition_duration(200);

        // Create widget fields first
        let username_entry = adw::EntryRow::builder().title("Username").build();

        let version_list_model = gtk::StringList::new(&[]);
        let version_combo = {
            let combo = adw::ComboRow::builder().title("Minecraft Version").build();
            combo.set_model(Some(&version_list_model));
            combo
        };
        model.version_list_model = Some(version_list_model.clone());

        let max_ram = crate::utils::get_total_memory_mb();
        let ram_scale = adw::SpinRow::builder()
            .title("RAM (MB)")
            .adjustment(&gtk::Adjustment::new(
                4096.0,
                2048.0,
                max_ram as f64,
                256.0,
                256.0,
                0.0,
            ))
            .build();

        let fabric_switch = adw::SwitchRow::builder()
            .title("Install Fabric")
            .subtitle("Install Fabric Modloader for this version")
            .build();

        let hide_logs_switch = adw::SwitchRow::builder().title("Hide Console").build();
        let discord_presence_switch =
            adw::SwitchRow::builder().title("Discord Rich Presence").build();

        let profile_list = gtk::ListBox::new();
        let loading_widgets = create_loading_widgets();

        // Create pages for each section
        let home_page = create_home_page(&sender, &profile_list);
        let create_page = create_create_instance_page(
            &sender,
            &username_entry,
            &version_combo,
            &ram_scale,
            &fabric_switch,
        );
        let profiles_page = create_profiles_page(model.rt.clone());
        let (settings_page, theme_combo) = create_settings_page(
            &sender,
            &hide_logs_switch,
            &discord_presence_switch,
        );
        let (logs_page, logs_view) = create_logs_page(&sender, &model.logs);

        content_stack.add_titled(&home_page, Some("home"), "Home");
        content_stack.add_titled(&create_page, Some("create"), "Create");
        content_stack.add_titled(&profiles_page, Some("profiles"), "Profiles");
        content_stack.add_titled(&settings_page, Some("settings"), "Settings");
        content_stack.add_titled(&logs_page, Some("logs"), "Logs");
        content_stack.add_titled(&loading_widgets.0, Some("loading"), "Loading");

        // Initialize Hide Logs State
        logs_button.set_visible(!model.settings.hide_logs);

        // Error Page
        let error_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();

        let error_label = gtk::Label::new(None);
        error_label.add_css_class("error-label");
        error_label.set_wrap(true);
        error_label.set_max_width_chars(50);
        error_label.set_halign(gtk::Align::Center);

        let back_button = gtk::Button::builder()
            .label("Back to Home")
            .halign(gtk::Align::Center)
            .css_classes(vec!["suggested-action".to_string()])
            .build();

        let sender_clone = sender.clone();
        back_button.connect_clicked(move |_| {
            sender_clone.input(AppMsg::BackToMainMenu);
        });

        error_box.append(&error_label);
        error_box.append(&back_button);

        let error_status_page = adw::StatusPage::builder()
            .title("Error")
            .icon_name("dialog-error-symbolic")
            .child(&error_box)
            .build();

        content_stack.add_titled(&error_status_page, Some("error"), "Error");

        // Wrap content_stack in a NavigationPage for NavigationSplitView
        let navigation_page = adw::NavigationPage::builder()
            .title("BRUZON's Launcher")
            .child(&content_stack)
            .build();

        navigation_split_view.set_content(Some(&navigation_page));

        // Toast Overlay
        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&navigation_split_view));
        model.toast_overlay = Some(toast_overlay.clone());

        // Create header bar
        let header_bar = adw::HeaderBar::new();
        header_bar.set_show_end_title_buttons(true);
        header_bar.set_title_widget(Some(&adw::WindowTitle::new("BRUZON's Launcher", "")));

        // Sidebar toggle button
        let sidebar_toggle_button = gtk::Button::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text("Toggle Sidebar")
            .build();

        let sender_clone = sender.clone();
        sidebar_toggle_button.connect_clicked(move |_| {
            sender_clone.input(AppMsg::ToggleSidebar);
        });

        header_bar.pack_start(&sidebar_toggle_button);

        // Create main container
        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        main_box.set_vexpand(true);
        main_box.set_hexpand(true);
        main_box.set_valign(gtk::Align::Fill);
        main_box.append(&header_bar);
        main_box.append(&toast_overlay);

        root.set_content(Some(&main_box));

        // Create widgets struct
        let widgets = AppWidgets {
            window: root.clone(),
            header_bar,
            navigation_split_view,
            navigation_page,
            content_stack,
            home_page,
            create_page,
            settings_page,
            logs_page,
            loading_page: loading_widgets.0,
            loading_spinner: loading_widgets.1,
            loading_progress: loading_widgets.2,
            loading_label: loading_widgets.3,

            profile_list,
            username_entry,
            version_combo,
            ram_scale,
            fabric_switch,

            hide_logs_switch,
            discord_presence_switch,
            launch_button: gtk::Button::with_label("Launch"),
            create_button: gtk::Button::with_label("Create"),
            delete_button: gtk::Button::with_label("Delete"),
            save_button: gtk::Button::with_label("Save"),
            cancel_button: gtk::Button::with_label("Cancel"),
            home_button,
            create_sidebar_button,
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
            sidebar_toggle_button,
            theme_combo,
            status_label: gtk::Label::new(None),
            error_label,

            toast_overlay,
            logs_view,
        };

        // Start loading data
        sender.input(AppMsg::NavigateToSection(Section::Home));

        // Load versions
        let sender_clone = sender.clone();
        if let Some(launcher) = &model.launcher {
            let launcher_clone = launcher.clone();
            model.rt.spawn(async move {
                match launcher_clone.get_available_versions().await {
                    Ok(versions) => sender_clone.input(AppMsg::VersionsLoaded(Ok(versions))),
                    Err(e) => sender_clone.input(AppMsg::VersionsLoaded(Err(e.to_string()))),
                }
            });
        }

        // Load settings
        let sender_clone = sender.clone();
        let config_dir_clone = if let Some(l) = &model.launcher {
            l.config.minecraft_dir.clone()
        } else {
            std::path::PathBuf::from(".")
        };
        model.rt.spawn(async move {
            let settings = Settings::load(&config_dir_clone).await;
            sender_clone.input(AppMsg::SettingsLoaded(settings));
        });

        // Load profiles
        let sender_clone = sender.clone();
        if let Some(launcher) = &model.launcher {
            let config_dir = launcher.config.minecraft_dir.clone();
            model.rt.spawn(async move {
                let path = config_dir.join("profiles.json");
                let profiles = if tokio::fs::try_exists(&path).await.unwrap_or(false) {
                    match tokio::fs::read_to_string(&path).await {
                        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                        Err(_) => HashMap::new(),
                    }
                } else {
                    HashMap::new()
                };
                sender_clone.input(AppMsg::ProfilesLoaded(Ok(profiles)));
            });
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        // Implementation of update logic
        match msg {
            AppMsg::NavigateToSection(section) => {
                self.state = AppState::Ready {
                    current_section: section,
                };
            }

            AppMsg::SettingsLoaded(settings) => {
                self.settings = settings.clone();
                // Apply loaded settings
                self.sidebar_collapsed = settings.sidebar_collapsed;
                self.sender
                    .input(AppMsg::ToggleHideLogs(settings.hide_logs));

                // Delay theme application to ensure window is fully realized or just apply it
                let theme = settings.theme.clone();
                let sender = self.sender.clone();
                // Apply immediately
                sender.input(AppMsg::ThemeSelected(theme));
                self.apply_discord_presence(RpcPresence::InLauncher);
            }
            AppMsg::ToggleHideLogs(hide) => {
                self.settings.hide_logs = hide;
                self.save_settings();
            }
            AppMsg::ToggleDiscordPresence(enabled) => {
                self.settings.discord_presence = enabled;
                self.save_settings();
                if enabled {
                    self.apply_discord_presence(RpcPresence::InLauncher);
                } else {
                    self.apply_discord_presence(RpcPresence::Disabled);
                }
            }
            AppMsg::ToggleSidebar => {
                self.sidebar_collapsed = !self.sidebar_collapsed;
                self.settings.sidebar_collapsed = self.sidebar_collapsed;
                self.save_settings();
            }
            AppMsg::Log(log_line) => {
                let mut end_iter = self.logs.end_iter();
                self.logs.insert(&mut end_iter, &format!("{}\n", log_line));
            }
            AppMsg::VersionsLoaded(result) => {
                match result {
                    Ok(versions) => {
                        use crate::utils::compare_versions;
                        let mut filtered: Vec<_> = versions.into_iter().collect();
                        filtered.sort_by(|a, b| compare_versions(&b.id, &a.id));

                        self.sorted_versions = filtered.iter().map(|v| v.id.clone()).collect();
                        self.available_versions = filtered;
                        self.versions_updated = true;

                        if let Some(string_list) = &self.version_list_model {
                            while string_list.n_items() > 0 {
                                string_list.remove(0);
                            }
                            for version in &self.sorted_versions {
                                string_list.append(version);
                            }
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to load versions: {}", e));
                    }
                }
            }
            AppMsg::ProfilesLoaded(result) => match result {
                Ok(profiles) => {
                    self.profiles = profiles;
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to load profiles: {}", e));
                }
            },
            AppMsg::LaunchProfile(profile_name) => {
                if let Some(profile) = self.profiles.get(&profile_name) {
                    if let Some(launcher) = &self.launcher {
                        let launcher_clone = launcher.clone();
                        let profile_clone = profile.clone();
                        let sender_clone = sender.clone();

                        self.state = AppState::Launching {
                            version: profile_clone.version.clone(),
                        };
                        let profile_name_clone = profile_name.clone();
                        self.apply_discord_presence(RpcPresence::Launching {
                            version: profile_clone.version.clone(),
                        });

                        let rt = self.rt.clone();
                        rt.spawn(async move {
                            let sender_progress = sender_clone.clone();
                            let on_progress = move |pct: f64, msg: String| {
                                sender_progress.input(AppMsg::DownloadProgress(pct, msg));
                            };

                            match launcher_clone
                                .prepare_and_launch(
                                    profile_clone.version.clone(),
                                    profile_clone.username.clone(),
                                    profile_clone.ram_mb,
                                    profile_clone.is_fabric,
                                    profile_clone
                                        .game_dir
                                        .as_ref()
                                        .map(std::path::PathBuf::from),
                                    on_progress,
                                )
                                .await
                            {
                                Ok(mut command) => match command.spawn() {
                                    Ok(mut child) => {
                                        sender_clone.input(AppMsg::GameStarted);
                                        let start_time = std::time::Instant::now();
                                        let stdout = child.stdout.take();
                                        let stderr = child.stderr.take();

                                        if let Some(stdout) = stdout {
                                            let sender_log = sender_clone.clone();
                                            let mut reader = BufReader::new(stdout).lines();
                                            tokio::spawn(async move {
                                                while let Ok(Some(line)) = reader.next_line().await {
                                                    sender_log.input(AppMsg::Log(line));
                                                }
                                            });
                                        }
                                        if let Some(stderr) = stderr {
                                            let sender_log = sender_clone.clone();
                                            let mut reader = BufReader::new(stderr).lines();
                                            tokio::spawn(async move {
                                                while let Ok(Some(line)) = reader.next_line().await {
                                                    sender_log.input(AppMsg::Log(format!("[ERR] {}", line)));
                                                }
                                            });
                                        }

                                        let _ = child.wait().await;
                                        let duration = start_time.elapsed().as_secs();
                                        sender_clone.input(AppMsg::SessionEnded(
                                            profile_name_clone,
                                            duration,
                                        ));
                                        sender_clone.input(AppMsg::LaunchCompleted);
                                    }
                                    Err(e) => sender_clone
                                        .input(AppMsg::Error(format!("Failed to spawn: {}", e))),
                                },
                                Err(e) => {
                                    sender_clone
                                        .input(AppMsg::Error(format!("Launch Failed: {}", e)));
                                }
                            }
                        });
                    }
                }
            }
            AppMsg::GameStarted => {
                if let AppState::Launching { version } = &self.state {
                    self.state = AppState::GameRunning {
                        version: version.clone(),
                    };
                }
                self.apply_discord_presence(RpcPresence::Clear);
            }
            AppMsg::DownloadProgress(progress, status) => {
                if let AppState::Downloading { version, .. } = &self.state {
                    self.state = AppState::Downloading {
                        version: version.clone(),
                        progress,
                        status,
                    };
                }
            }
            AppMsg::LaunchCompleted => {
                self.state = AppState::Ready {
                    current_section: Section::Home,
                };
                self.apply_discord_presence(RpcPresence::InLauncher);
            }
            AppMsg::UsernameChanged(username) => {
                self.input_username = username;
            }
            AppMsg::RamChanged(ram) => {
                self.input_ram = ram;
            }
            AppMsg::VersionSelected(version) => {
                use crate::utils::is_at_least_1_14;
                if is_at_least_1_14(&version) {
                    self.fabric_switch_enabled = true;
                } else {
                    self.fabric_switch_enabled = false;
                    self.input_install_fabric = false;
                }
                self.input_version = Some(version);
            }
            AppMsg::ToggleFabric(install) => {
                self.input_install_fabric = install;
            }
            AppMsg::SaveProfile => {
                if self.input_username.trim().is_empty() {
                    return;
                }
                if self.input_version.is_none() {
                    return;
                }

                let selected_version = self.input_version.clone().unwrap();
                let is_fabric = self.input_install_fabric && self.fabric_switch_enabled;

                let profile = Profile {
                    username: self.input_username.clone(),
                    version: selected_version.clone(),
                    ram_mb: self.input_ram,
                    playtime_seconds: 0,
                    last_launch: None,
                    is_fabric,
                    game_dir: None,
                };

                let profile_name = if is_fabric {
                    format!("{}_{}_fabric", profile.username, profile.version)
                } else {
                    format!("{}_{}", profile.username, profile.version)
                };

                self.profiles.insert(profile_name.clone(), profile);

                self.save_profiles(sender.clone());

                self.input_username.clear();
                self.input_version = None;
                self.input_ram = 4096;
                self.input_install_fabric = false;
                self.fabric_switch_enabled = false;

                sender.input(AppMsg::NavigateToSection(Section::Home));
            }
            AppMsg::DeleteProfile(profile_name) => {
                self.profiles.remove(&profile_name);
                self.save_profiles(sender.clone());
                sender.input(AppMsg::NavigateToSection(Section::Home));
            }
            AppMsg::BackToMainMenu => {
                sender.input(AppMsg::NavigateToSection(Section::Home));
            }

            AppMsg::Error(message) => {
                self.state = AppState::Error { message };
            }
            AppMsg::ThemeSelected(theme) => {
                self.settings.theme = theme.clone();
                if let Some(window) = &self.window {
                    let style_manager = adw::StyleManager::default();
                    window.remove_css_class("transparent-window");

                    match theme {
                        Theme::Dark => style_manager.set_color_scheme(adw::ColorScheme::ForceDark),
                        Theme::Light => {
                            style_manager.set_color_scheme(adw::ColorScheme::ForceLight)
                        }
                        Theme::System => style_manager.set_color_scheme(adw::ColorScheme::Default),
                        Theme::Transparent => {
                            style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
                            window.add_css_class("transparent-window");
                        }
                    }
                }
                self.save_settings();
            }
            AppMsg::OpenMinecraftFolder => {
                if let Some(launcher) = &self.launcher {
                    let dir = launcher.config.minecraft_dir.clone();
                    self.rt.spawn(async move {
                        let _ = open::that(dir);
                    });
                }
            }
            AppMsg::RequestDeleteProfile(profile_name) => {
                if let Some(window) = &self.window {
                    let dialog = adw::MessageDialog::builder()
                        .heading("Delete Profile?")
                        .body(format!(
                            "Are you sure you want to delete profile '{}' ?",
                            profile_name
                        ))
                        .transient_for(window)
                        .modal(true)
                        .build();
                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("delete", "Delete");
                    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                    let sender_clone = sender.clone();
                    let pname = profile_name.clone();
                    dialog.connect_response(None, move |d, response| {
                        if response == "delete" {
                            sender_clone.input(AppMsg::DeleteProfile(pname.clone()));
                        }
                        d.close();
                    });
                    dialog.present();
                }
            }
            AppMsg::SessionEnded(profile_name, duration) => {
                if let Some(profile) = self.profiles.get_mut(&profile_name) {
                    profile.playtime_seconds += duration;
                    profile.last_launch = Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    );
                    self.save_profiles(sender.clone());
                }
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        match &self.state {
            AppState::Loading => {
                widgets.content_stack.set_visible_child_name("loading");
                widgets.loading_spinner.start();
            }
            AppState::Ready { current_section } => {
                widgets.loading_spinner.stop();
                widgets.set_sidebar_buttons_sensitive(true);
                widgets.clear_sidebar_selection();

                match current_section {
                    Section::Home => {
                        widgets.home_button.add_css_class("suggested-action");
                        widgets.content_stack.set_visible_child_name("home");
                        update_profile_list(&widgets.profile_list, &self.profiles, &self.sender);
                    }
                    Section::Profiles => {
                        widgets.content_stack.set_visible_child_name("profiles");
                    }
                    Section::CreateInstance => {
                        widgets
                            .create_sidebar_button
                            .add_css_class("suggested-action");
                        widgets.content_stack.set_visible_child_name("create");
                        widgets.fabric_switch.set_active(self.input_install_fabric);
                        widgets
                            .fabric_switch
                            .set_sensitive(self.fabric_switch_enabled);
                    }
                    Section::Settings => {
                        widgets.settings_button.add_css_class("suggested-action");
                        widgets.content_stack.set_visible_child_name("settings");
                    }
                    Section::Logs => {
                        widgets.logs_button.add_css_class("suggested-action");
                        widgets.content_stack.set_visible_child_name("logs");
                    }
                }
            }
            AppState::Downloading {
                progress, status, ..
            } => {
                widgets.content_stack.set_visible_child_name("loading");
                widgets.loading_page.set_title("Downloading...");
                widgets.loading_page.set_description(Some(status));
                widgets
                    .loading_page
                    .set_child(Some(&widgets.loading_progress));
                widgets.loading_progress.set_fraction(*progress);
                widgets.loading_spinner.stop();
                widgets.set_sidebar_buttons_sensitive(false);
            }
            AppState::Launching { .. } => {
                widgets.content_stack.set_visible_child_name("loading");
                widgets.loading_page.set_title("Launching...");
                widgets.loading_page.set_description(Some("If this is your first time launching, it may take longer as files are downloaded."));
                widgets
                    .loading_page
                    .set_child(Some(&widgets.loading_spinner));
                widgets.loading_spinner.start();
                widgets.set_sidebar_buttons_sensitive(false);
            }
            AppState::GameRunning { .. } => {
                widgets.content_stack.set_visible_child_name("loading");
                widgets.loading_page.set_title("Game Running");
                widgets
                    .loading_page
                    .set_description(Some("Minecraft is running."));
                widgets
                    .loading_page
                    .set_child(Some(&widgets.loading_spinner));
                widgets.loading_spinner.start();
                widgets.set_sidebar_buttons_sensitive(false);
            }
            AppState::Error { message } => {
                widgets.error_label.set_text(message);
                widgets.content_stack.set_visible_child_name("error");
            }
        }

        widgets.logs_button.set_visible(!self.settings.hide_logs);
        widgets.hide_logs_switch.set_active(self.settings.hide_logs);
        widgets
            .discord_presence_switch
            .set_active(self.settings.discord_presence);

        let theme_index = match self.settings.theme {
            Theme::System => 0,
            Theme::Light => 1,
            Theme::Dark => 2,
            Theme::Transparent => 3,
        };
        if widgets.theme_combo.selected() != theme_index {
            widgets.theme_combo.set_selected(theme_index);
        }

        if self.sidebar_collapsed {
            widgets.navigation_split_view.set_min_sidebar_width(60.0);
            widgets.navigation_split_view.set_max_sidebar_width(60.0);
            widgets.home_box.set_halign(gtk::Align::Center);
            widgets.create_box.set_halign(gtk::Align::Center);
            widgets.settings_box.set_halign(gtk::Align::Center);
            widgets.logs_box.set_halign(gtk::Align::Center);
        } else {
            widgets.navigation_split_view.set_min_sidebar_width(180.0);
            widgets.navigation_split_view.set_max_sidebar_width(250.0);
            widgets.home_box.set_halign(gtk::Align::Start);
            widgets.create_box.set_halign(gtk::Align::Start);
            widgets.settings_box.set_halign(gtk::Align::Start);
            widgets.logs_box.set_halign(gtk::Align::Start);
        }

        widgets.home_label.set_visible(!self.sidebar_collapsed);
        widgets.create_label.set_visible(!self.sidebar_collapsed);
        widgets.settings_label.set_visible(!self.sidebar_collapsed);
        widgets.logs_label.set_visible(!self.sidebar_collapsed);
    }
}

impl AppModel {
    fn apply_discord_presence(&self, presence: RpcPresence) {
        if self.settings.discord_presence {
            self.discord_rpc.update(presence);
        } else {
            self.discord_rpc.update(RpcPresence::Disabled);
        }
    }

    fn save_settings(&self) {
        if let Some(launcher) = &self.launcher {
            let config_dir = launcher.config.minecraft_dir.clone();
            let settings_clone = self.settings.clone();
            std::thread::spawn(move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let _ = settings_clone.save(&config_dir).await;
                });
            });
        }
    }

    fn save_profiles(&self, sender: ComponentSender<Self>) {
        if let Some(launcher) = &self.launcher {
            let config_dir = launcher.config.minecraft_dir.clone();
            let profiles_clone = self.profiles.clone();
            std::thread::spawn(move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let path = config_dir.join("profiles.json");
                    let json = serde_json::to_string_pretty(&profiles_clone).unwrap_or_default();
                    if let Err(e) = tokio::fs::write(&path, json).await {
                        sender.input(AppMsg::Error(format!("Failed to save profiles: {}", e)));
                    }
                });
            });
        }
    }
}

impl AppWidgets {
    fn set_sidebar_buttons_sensitive(&self, sensitive: bool) {
        self.home_button.set_sensitive(sensitive);
        self.create_sidebar_button.set_sensitive(sensitive);
        self.settings_button.set_sensitive(sensitive);
        self.logs_button.set_sensitive(sensitive);
    }

    fn clear_sidebar_selection(&self) {
        self.home_button.remove_css_class("suggested-action");
        self.create_sidebar_button
            .remove_css_class("suggested-action");
        self.settings_button.remove_css_class("suggested-action");
        self.logs_button.remove_css_class("suggested-action");
    }
}
