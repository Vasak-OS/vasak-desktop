<p align="center">
  <img width="200" src="https://icon.icepanel.io/Technology/svg/Tauri.svg" style="block-display: inline;">
   <img width="200" src="https://icon.icepanel.io/Technology/svg/Vue.js.svg" style="block-display: inline;">
   <img width="200" src="https://icon.icepanel.io/Technology/svg/TypeScript.svg" style="block-display: inline;">
   <img width="200" src="https://icon.icepanel.io/Technology/svg/Bun.svg" style="block-display: inline;">
</p>
<br />
Vasak Desktop is a modern desktop environment built with Tauri (Rust backend + Vue.js frontend) that provides a comprehensive Linux desktop shell. It implements a multi-window desktop interface with system panel, application launcher, control center, and desktop background management across multiple monitors.

This document covers the overall system architecture, core components, and technology stack.

## Getting Started

### Prerequisites and System Requirements
The vasak-desktop application requires both Node.js/JavaScript and Rust toolchains due to its hybrid Tauri architecture.

#### Required Dependencies
* JavaScript Runtime (Bun Recommended):
  * Bun runtime
  * Package manager: bun
* Rust Toolchain:
  * Rust 1.70+ with Cargo
  * Tauri CLI 2.8+
* System Libraries: The application requires system libraries for desktop integration:
  * GTK 3.0+ development libraries
  * D-Bus development libraries
  * X11 development libraries (for X11 support)
  * Wayland development libraries (for Wayland support)

### Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) 
- [Volar](https://marketplace.visualstudio.com/items?itemName=Vue.volar)
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## System Architecture

Vasak Desktop follows a hybrid web-native architecture where Vue.js handles the user interface layer while Rust manages system-level operations through Tauri's IPC bridge.

## Technology Stack
### Build and Development Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Frontend Framework | Vue.js 3.5.18 | Reactive user interface |
| Backend Runtime | Rust + Tauri 2.x | System integration and window management |
| Styling | Tailwind CSS 4.1.12 | Utility-first CSS framework |
| Build Tool | Vite 7.1.3 | Frontend build and development server |
| State Management | Pinia 3.0.3 | Vue.js state management |
| Routing | Vue Router 4.5.1 | Client-side routing |
| Type System | TypeScript 5.9.2 | Static type checking |

### System Integration Technologies
| Component | Technology | Purpose |
|-----------|------------|---------|
| Display Server | X11 + Wayland | Multi-protocol display server support |
| GUI Toolkit | GTK 0.18 + GDK 0.18 | Native Linux widget integration |
| IPC/DBus | zbus 4.x | D-Bus communication for system services |
| Image Processing | image 0.25 | Icon and image handling |
| Desktop Entries | freedesktop_entry_parser 1.3 | .desktop file parsing |
| Async Runtime | Tokio 1.x | Asynchronous task execution |