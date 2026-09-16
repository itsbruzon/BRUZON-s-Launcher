<div align="center">

# BRUZON's Launcher

**BLauncher — A Modern, Lightweight Minecraft Launcher for Linux**

[![Rust](https://img.shields.io/badge/Made_with-Rust-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![GTK4](https://img.shields.io/badge/GUI-GTK4-blue?style=flat-square&logo=gtk)](https://www.gtk.org/)
[![License](https://img.shields.io/badge/License-GPL_v3-green?style=flat-square)](LICENSE)
[![Version](https://img.shields.io/badge/Version-1.1.0-purple?style=flat-square)]()

</div>

---

**BRUZON's Launcher (BLauncher)** is a modern, lightweight, open-source Minecraft launcher for Linux, built with **Rust**, **GTK4**, **Libadwaita**, and **Relm4**.

BLauncher is designed to provide a fast, native Linux experience for managing and launching Minecraft. The project is based on the open-source RCraft codebase and is being developed as an independent launcher with its own identity, interface, and future feature set.

## Features

- **Native Linux UI**: Built with GTK4 and Libadwaita for a responsive desktop experience.
- **Minecraft Version Management**: Download and launch Minecraft release versions through Mojang's version metadata.
- **Profile System**: Create and manage multiple Minecraft profiles with custom usernames, versions, RAM allocation, and Fabric support.
- **Fabric Support**: Install and launch Fabric for supported Minecraft versions.
- **Java Detection**: Automatically locate an available Java runtime on the system.
- **Library & Asset Management**: Download Minecraft libraries, native libraries, and game assets as required.
- **Discord Rich Presence**: Optional Discord Rich Presence integration while using the launcher and starting Minecraft.
- **Launch Logs**: View Minecraft output and errors directly inside the launcher.
- **Lightweight**: Written in Rust with a focus on a small, responsive native application.

## Project Status

BLauncher is actively being developed. The current codebase provides the core launcher functionality inherited from the original RCraft foundation, while the project is being redesigned and expanded under the BLauncher identity.

Planned development includes improvements to instance management, UI design, Java/runtime management, mod-loader support, download reliability, and overall launcher architecture.

## Usage

BLauncher is intended to be distributed as an **AppImage**, providing a convenient way to run the launcher on Linux without a traditional installation.

### Quick Start

1. Download the latest **BLauncher AppImage** from the [Releases](https://github.com/itsbruzon/BRUZON-s-Launcher/releases) page.
2. Make it executable:

   ```bash
   chmod +x BLauncher-x86_64.AppImage
   ```

3. Run it:

   ```bash
   ./BLauncher-x86_64.AppImage
   ```

   You can also launch the AppImage from your file manager.

## Building from Source

BLauncher is written in Rust and uses GTK4, Libadwaita, and Relm4.

After installing the required Linux development dependencies and Rust toolchain, clone the repository and build it with Cargo:

```bash
git clone https://github.com/itsbruzon/BRUZON-s-Launcher.git
cd BRUZON-s-Launcher
cargo build --release
```

The resulting release binary will be located at:

```text
target/release/BLauncher
```

## License

Distributed under the **GPL-3.0 License**. See [LICENSE](LICENSE) for more information.

---

<div align="center">
  Created and maintained by <a href="https://github.com/itsbruzon">BRUZON</a>
</div>
