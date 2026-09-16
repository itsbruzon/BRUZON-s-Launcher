//! Discord Rich Presence using Minecraft's official Discord application ID.
use discordipc::activity::{Activity, ActivityType, Assets, Timestamps};
use discordipc::packet::Packet;
use discordipc::Client;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// Official Minecraft Java Edition Discord application ID.
pub const MINECRAFT_DISCORD_APP_ID: &str = "155015894178529280";

#[derive(Debug, Clone)]
pub enum RpcPresence {
    Disabled,
    InLauncher,
    Launching { version: String },
    Clear,
}

#[derive(Clone)]
pub struct MinecraftRpc {
    tx: mpsc::Sender<RpcPresence>,
}

impl MinecraftRpc {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || rpc_worker(rx));
        Self { tx }
    }

    pub fn update(&self, presence: RpcPresence) {
        let _ = self.tx.send(presence);
    }
}

impl Drop for MinecraftRpc {
    fn drop(&mut self) {
        let _ = self.tx.send(RpcPresence::Clear);
    }
}

fn rpc_worker(rx: mpsc::Receiver<RpcPresence>) {
    let client = Client::new(MINECRAFT_DISCORD_APP_ID);
    let mut connected = false;

    while let Ok(presence) = rx.recv() {
        match presence {
            RpcPresence::Disabled => {
                if connected {
                    let _ = client.send_and_wait(Packet::new_activity(None, None));
                    let _ = client.disconnect();
                    connected = false;
                }
            }
            RpcPresence::Clear => {
                if connected {
                    let _ = client.send_and_wait(Packet::new_activity(None, None));
                }
            }
            RpcPresence::InLauncher => {
                if !ensure_connected(&client, &mut connected) {
                    continue;
                }
                let activity = launcher_activity("In the launcher");
                let _ = client.send_and_wait(Packet::new_activity(Some(&activity), None));
            }
            RpcPresence::Launching { version } => {
                if !ensure_connected(&client, &mut connected) {
                    continue;
                }
                let activity = Activity::new()
                    .kind(ActivityType::Playing)
                    .details("Minecraft")
                    .state(format!("Starting {}", version))
                    .timestamps(Timestamps::new().start_now())
                    .assets(minecraft_assets());
                let _ = client.send_and_wait(Packet::new_activity(Some(&activity), None));
            }
        }
    }

    if connected {
        let _ = client.disconnect();
    }
}

fn launcher_activity(state: &str) -> Activity {
    Activity::new()
        .kind(ActivityType::Playing)
        .details("Minecraft")
        .state(state)
        .timestamps(Timestamps::new().start_now())
        .assets(minecraft_assets())
}

fn minecraft_assets() -> Assets {
    Assets::new().large_image("minecraft", Some("Minecraft"))
}

fn ensure_connected(client: &Client<Arc<discordipc::InnerClient>>, connected: &mut bool) -> bool {
    if *connected {
        return true;
    }
    match client.connect_and_wait() {
        Ok(response) => {
            if response.filter().is_ok() {
                *connected = true;
                true
            } else {
                false
            }
        }
        Err(_) => false,
    }
}
