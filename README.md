# Vasak desktop

La magia de VasakOS el core de su Desktop, TopBar, Menu y Mas... Todo diseñado con VueJS y Tauri para VasakOS

---

## Tecnología

[![Rust](https://img.shields.io/badge/Rust-1.82%2B-orange?style=flat&logo=rust)](https://www.rust-lang.org/) [![Tauri](https://img.shields.io/badge/Tauri-2.0-24283b?style=flat&logo=tauri)](https://tauri.app/) [![Vue](https://img.shields.io/badge/Vue-3.5-42b883?style=flat&logo=vue.js)](https://vuejs.org/) [![TypeScript](https://img.shields.io/badge/TypeScript-5.6-3178c6?style=flat&logo=typescript)](https://www.typescriptlang.org/) [![systemd](https://img.shields.io/badge/systemd-user-24283b?style=flat&logo=systemd)](https://systemd.io/)

## Estado

[![CI](https://github.com/Vasak-OS/vasak-desktop/actions/workflows/ci.yml/badge.svg)](https://github.com/Vasak-OS/vasak-desktop/actions/workflows/ci.yml)
[![Versión](https://img.shields.io/github/v/release/Vasak-OS/vasak-desktop?style=flat&label=Versi%C3%B3n)](https://github.com/Vasak-OS/vasak-desktop/releases)
[![Licencia](https://img.shields.io/github/license/Vasak-OS/vasak-desktop?style=flat)](LICENSE)
[![Cobertura](https://sonarcloud.io/api/project_badges/measure?project=Vasak-OS_vasak-desktop&metric=coverage&style=flat)](https://sonarcloud.io/summary/new_code?id=Vasak-OS_vasak-desktop)
[![Quality Gate](https://sonarcloud.io/api/project_badges/measure?project=Vasak-OS_vasak-desktop&metric=alert_status&style=flat)](https://sonarcloud.io/summary/new_code?id=Vasak-OS_vasak-desktop)
[![Tamaño repo](https://img.shields.io/github/repo-size/Vasak-OS/vasak-desktop?style=flat)](https://github.com/Vasak-OS/vasak-desktop)

---

## Descripción

La magia de VasakOS el core de su Desktop, TopBar, Menu y Mas... Todo diseñado con VueJS y Tauri para VasakOS

**Para el usuario:** Todo diseñado con VueJS y Tauri para VasakOS

**Integración VasakOS:** Integra con el ecosistema VasakOS vía D-Bus y configuración compartida.

---

## Arquitectura (diagramas)

> Copia y pega la plantilla que necesites, descomenta y adapta.

### Flujo de datos

```mermaid
graph TD
    A[Entrada: Teclado / Ratón / D-Bus / Sistema] --> B[Procesamiento: Core Rust + Tauri Commands]
    B --> C[Salida: UI actualizada / Acciones del sistema]
    style A fill:#e1f5fe
    style C fill:#e8f5e9
```

### Componentes internos

```mermaid
flowchart LR
    subgraph App[Vasak desktop]
        UI[Frontend Vue 3 + vue-libvasak]
        Core[Core Rust: panel, overview, notifications, media + daemon systemd]
        IPC[Tauri Commands / Eventos + D-Bus]
    end
    UI --> IPC
    IPC --> Core
    Core --> DBus[(D-Bus / zbus)]
    Core --> System[Systemd --user / D-Bus / Wayfire IPC / PipeWire]
    style UI fill:#f3e5f5
    style Core fill:#e3f2fd
    style DBus fill:#fff3e0
    style System fill:#fce4ec
```

### Integración en el sistema VasakOS

```mermaid
C4Context
    title Vasak desktop en VasakOS
    Person(usuario, "Usuario", "Persona que usa el sistema")
    System_Boundary(c1, "VasakOS") {
        System(app, "Vasak desktop", "La magia de VasakOS el core de su Desktop, TopBar, Menu y Mas... Todo diseñado ")
        System_Ext(settings, "vasak-settings", "Configuración central")
        System_Ext(keyring, "vasak-keyring", "Secrets y claves")
        System_Ext(accounts, "vasak-accounts", "Cuentas e identidad")
        System_Ext(permissions, "vasak-permissions", "Permisos y portales")
    }
    System_Ext(systemd, "systemd --user", "Socket-activated via D-Bus")
    System_Ext(dbus, "D-Bus sesión", "Bus de mensajes")
    System_Ext(wayland, "Wayland / Wayfire", "Compositor + plugins")
    System_Ext(pipewire, "PipeWire", "Control de medios MPRIS")
    Rel(usuario, app, "Usa")
    Rel(app, settings, "Lee/escribe config")
    Rel(app, keyring, "Guarda/lee secrets")
    Rel(app, accounts, "Identidad de cuenta")
    Rel(app, permissions, "Solicita permisos")
    Rel(app, systemd, "Socket-activated via D-Bus")
    Rel(app, dbus, "Expone interfaces D-Bus")
    Rel(app, wayland, "Wayfire IPC: workspaces, ventanas")
    Rel(app, pipewire, "MPRIS: metadatos y controles")
```

---

## Instalación

### Arch Linux (repo oficial)

```bash
pacman -S vasak-desktop
```

### Debian / Ubuntu (.deb)

```bash
wget https://github.com/Vasak-OS/vasak-desktop/releases/download/v{{VERSION}}/vasak-desktop_{{VERSION}}_amd64.deb
sudo apt install ./vasak-desktop_{{VERSION}}_amd64.deb
```

### Desde fuente

```bash
git clone https://github.com/Vasak-OS/vasak-desktop.git
cd vasak-desktop

# Desarrollo (recarga en caliente frontend + backend)
bun run dev

# Build producción
bun run build
```

### Flatpak / AppImage

> *Placeholder: añadir cuando exista paquete Flatpak o AppImage oficial.*

---

## Uso rápido

```bash
# Se lanza automáticamente al iniciar la sesión Wayfire (systemd --user socket-activated)
# Para lanzar manualmente (debug):
vasak-desktop
```

### Atajos / flags principales

| Acción | Comando / Tecla |
|--------|-----------------|
| Acción principal | `vasak-desktop` |
| Configuración | `vasak-desktop --config` |
| Ayuda | `vasak-desktop --help` |

> **Captura:** `assets/demo.gif` *(placeholder — sustituir por GIF real)*

---

## Configuración

### Archivo de configuración

```
~/.config/vasak/desktop.toml
```

Ejemplo mínimo:

```toml
[panel]
position = "top"
height = 36
autohide = false

[overview]
blur_background = true
show_recent = true
max_recent = 8

[notifications]
position = "right"
width = 360
max_visible = 10

[media]
show_artwork = true
preferred_players = ["spotify", "firefox", "mpv"]
```

### Variables de entorno

| Variable | Descripción | Por defecto |
|----------|-------------|-------------|
| `VASAK_DEBUG` | Activa logs de depuración en consola | `false` |
| `VASAK_PROFILE` | Perfil de rendimiento: balanced, performance, power-save | `balanced` |

### Integración con `tauri-plugin-config-manager`

- Esquemas de color: respeta `--ui-primary`, `--ui-surface`, `--ui-focus`, `--ui-on-surface`, `--ui-border`
- Tema de iconos: usa el tema del sistema (`vasakos-icon-theme`) vía `vicon` lookup
- Radio de bordes: variable CSS `--ui-radius` del esquema activo (por defecto 12px)
- Fuentes: `--ui-font-family`, `--ui-font-size-base` desde el esquema

---

## Desarrollo

### Requisitos

- **Rust:** `rustup` toolchain estable (`rust-toolchain.toml` fija la versión)
- **Node / bun:** `bun` ≥ 1.1 (nunca npm/yarn/pnpm para apps Tauri)
- **Sistema:** `pkg-config`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libssl-dev`
- *`libgtk-layer-shell-dev`, `wayland-protocols`, `wayfire-dev` para Wayland*

### Dependencias de desarrollo

#### Sistema (paquetes Arch / Debian / Fedora)

`base-devel`, `pkgconf`, `libssl-dev`, `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `gstreamer1.0-plugins-base`, `gstreamer1.0-plugins-good`, `libpipewire-0.3-dev`, `libwireplumber-0.5-dev`, `libdbus-1-dev`, `libudev-dev`, `libxkbcommon-dev`, `libwayland-dev`, `meson`, `ninja`, `clang`, `lld`

#### Rust

`rustup` → `stable` + `rustfmt` + `clippy`

#### Node

**Solo `bun`** (nunca npm/yarn/pnpm para apps Tauri). `bun --version` ≥ 1.1

#### Herramientas CLI

`just` (command runner), `cargo-binstall` (opcional), `cargo-llvm-cov` (para cobertura)

### Correr en desarrollo

```bash
# Frontend (Vite dev server + Tauri dev)
bun run tauri dev

# Solo frontend (Vite)
bun run dev

# Solo backend (cargo watch)
cargo watch -x "run --bin vasak-desktop`
```

### Compilar / Build de release

```bash
# Build completo (frontend + backend + bundle)
bun run tauri build

# Solo frontend (para testear bundle después)
bun run build

# Solo backend Rust (binario optimizado)
cargo build --release --locked

# Paquetes nativos
# Arch: makepkg -sf en PKGBUILDS/vasak-desktop/
# Debian: cargo deb --no-build --target-dir src-tauri/target/release/debian
```

### Targets principales

```bash
# Desarrollo
bun run dev

# Build producción
bun run build

# Solo frontend
bun run build:frontend

# Solo backend Rust
cargo build --release --locked

# Lint + tipos (pre-commit / CI)
bunx --bun biome check .
bunx vue-tsc --noEmit
cargo fmt --check
```

### Pruebas

```bash
# Suite completa JS (Vitest via bun:test)
bun test

# Suite completa Rust (crates en pruebas_rust del CI)
cargo test -p vasak-desktop-core -p vasak-desktop-ui --locked

# Integración (requiere Wayfire corriendo)
cargo test -p vasak-desktop-integration --locked -- --ignored
```

### Convención de commits

[Conventional Commits](https://www.conventionalcommits.org/es/v1.0.0/):

```
<tipo>(<ámbito>): <descripción corta>

[cuerpo opcional]

[pie opcional: Closes #N]
```

Tipos habituales: `feat`, `fix`, `bugfix`, `docs`, `ci`, `chore`, `security`, `refactor`, `perf`, `test`.

### Cómo abrir PR

1. Rama desde `origin/main`:
   ```bash
   git worktree add -b feat/{{TEMA}} ~/wt/vasak-desktop-{{TEMA}} origin/main
   ```
2. Prefijos de rama: `fix/`, `feat/`, `bugfix/`, `docs/`, `ci/`, `chore/`, `security/`
3. Escribe el fix → **`qa` escribe las pruebas** (fallan sin el fix) → suite verde → abre PR
4. Al abrir PR, **cuatro cosas obligatorias**:
   - Asignado a `JoaquinDecima`
   - Etiqueta `enhancement` o `bug` (+ `ci` / `dependencies` si aplica)
   - Añadido al proyecto «VasakOS Roadmap»
   - Status **In Progress**

---

## Arquitectura (texto)

### Crates / módulos principales

| Crate / Módulo | Responsabilidad |
|----------------|-----------------|
| `vasak-desktop-core` | Lógica compartida: estado, D-Bus, IPC |
| `vasak-desktop-ui` | Frontend Vue 3 + componentes vue-libvasak |
| `vasak-desktop-tauri` | Puente Tauri: commands, eventos, WebView |

### Interfaces D-Bus / zbus expuestas

| Interfaz | Métodos / Señales / Propiedades | Descripción |
|----------|----------------------------------|-------------|
| `org.vasak.Vasak-desktop` | `GetState()`, `StateChanged(dict)`, `SetConfig(dict)` | Estado y control principal |
| `org.vasak.Vasak-desktop.Panel` | `SetPosition(string)`, `SetAutohide(bool)`, `Height(u32)` | Configuración panel en runtime |

### Puntos de extensión

- **Plugins:** Sistema de plugins modular
- **Hooks:** Hooks de ciclo de vida
- **API pública Rust:** API pública del crate principal

### Features / Cargo features

| Feature | Descripción | Default |
|---------|-------------|---------|
| `system-tray` | Bandeja del sistema + menú contextual | ✓ |
| `notifications` | Integración libnotify / freedesktop | ✓ |
| `updater` | Auto-actualización (tauri-plugin-updater) | |
| `devtools` | DevTools en release (solo debug) | |
| `experimental-wayland` | Protocolo Wayland inestable | |

Y para apps Tauri+Vue: features del `tauri.conf.json` → `bundle` / `plugins` / `allowlist` (capabilities).

---

## Pruebas y calidad

- **Cobertura objetivo:** ≥ 80 % del código nuevo (SonarCloud Quality Gate)
- **Informes:** GitHub Actions → pestaña *SonarCloud Code Analysis* → enlace a SonarCloud
- **Añadir pruebas:** ver `AGENTS.md` sección 9 (mapa de repos y comandos de prueba)
- **Regla:** toda corrección lleva prueba escrita por `qa` que falla sin el fix

---

## Contribuir

- **Guía:** [`.github/CONTRIBUTING.md`](https://github.com/Vasak-OS/.github/blob/main/CONTRIBUTING.md)
- **Plantillas de issue:** [`.github/ISSUE_TEMPLATE/`](https://github.com/Vasak-OS/.github/tree/main/ISSUE_TEMPLATE)
- **Código de conducta:** [`CODE_OF_CONDUCT.md`](https://github.com/Vasak-OS/.github/blob/main/CODE_OF_CONDUCT.md)

---

## Contribuidores

> **Actualizado automáticamente en CI** — muestra los avatares de quienes han aportado código a este repo.

<div align="center">
  <a href="https://github.com/JoaquinDecima" target="_blank"><img src="https://avatars.githubusercontent.com/u/11674229?v=4&s=40" width="40" height="40" alt="JoaquinDecima" style="border-radius: 50%; margin: 2px;" title="JoaquinDecima (1179 contribuciones)"></a>
</div>

[Ver todos los contribuidores](https://github.com/Vasak-OS/vasak-desktop/graphs/contributors)

---

## Donadores / Sponsors

[![Sponsors](https://img.shields.io/github/sponsors/Vasak-OS?style=flat&label=Sponsors)](https://github.com/sponsors/Vasak-OS)

> **Backers:** *Actualizado automáticamente en CI desde GitHub Sponsors / OpenCollective / Polar.*

[💖 Donar al proyecto](https://github.com/sponsors/Vasak-OS)

---

## Licencia

`GPL-3.0` — Copyright © 2026 **Vasak-OS Contributors**

[Ver licencia completa](LICENSE)
