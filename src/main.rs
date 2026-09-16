mod auth;
mod config;
mod instances;
mod java_manager;
mod launcher;
mod minecraft_rpc;
mod library_manager;
mod models;
mod profiles;
mod settings;
mod ui;
mod utils;

use adw::Application;
use gtk4::glib;
use relm4::RelmApp;

use ui::AppModel;

fn main() {
    let app = Application::builder()
        .application_id("dev.bruzon.BLauncher")
        .build();

    glib::set_application_name("BRUZON's Launcher");

    let relm_app = RelmApp::from_app(app);
    relm_app.run::<AppModel>(())
}
