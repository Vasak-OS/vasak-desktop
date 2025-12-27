# VasakOS Desktop - Funcionalidades Faltantes 📋

**Última actualización**: 27 de Diciembre de 2025  
**Progreso general**: ~50% completado (↑ desde 45%)

## 🚀 Prioridades Inmediatas (Q1)

- [x] Timeouts y reintentos D-Bus
	- Aplica a música, batería, red, bluetooth.
	- Retry exponencial (3 intentos) y fallback a polling cuando falle conexión.
	- Logs estructurados y feedback UI discreto.
- [x] Manejo graceful de desconexiones
	- Detectar pérdida de conexión (UPower, NetworkManager, MPRIS) y reconectar.
	- No bloquear UI; mostrar estado “Reconectando…”.
- [x] Validación de estado pre-comandos
	- Comandos de música/red/bluetooth validan destino existe y está listo.
	- Respuestas coherentes con mensajes de error amigables.
- [x] Búsqueda global de archivos (MVP)
	- Indexación básica (home) en background, resultados por nombre y extensión.
	- Atajo panel: abrir búsqueda con resultados paginados.
- [x] Gestor de atajos de teclado (MVP)
	- Listar atajos actuales + editar 5 acciones clave.
	- Persistencia en config.
- ~~[ ] Tiling Window Manager (básico)~~
	- ~~Atajos para split left/right, stack, focus, move.~~
	- ~~Indicador visual de layout.~~
- [ ] UI de VPN en Network
	- Listar perfiles, conectar/desconectar, estado.
	- Soporte NetworkManager (nmcli) inicial.
- [x] Panel de Información del Sistema (MVP)
	- CPU/GPU/RAM uso, temperatura si disponible.
	- Uptime, kernel, entorno gráfico.
	- [NUEVO] Información de Swap y Discos
- [x] Configuración del Sistema Operativo (MVP)
	- Border radius ajustable (1-20px)
	- Colores primario y de énfasis personalizables
	- Toggle dark mode / light mode
	- Selector de tema GTK
	- Selector de cursor
	- Selector de pack de iconos
	- Persistencia en `~/.config/vasak/system_config.json`

## ✅ Criterios de Aceptación (DoD)

- Errores manejados y comunicados sin bloquear la UI.
- Logs estructurados (nivel info/warn/error) sin spam.
- Persistencia donde aplique (config, estado) con valores por defecto.
- Pruebas manuales documentadas (pasos, esperado/observado).
- Sin regresiones visibles en panel/control center.

## 🎵 **MÚSICA** (Prioridad: ALTA)
### ✅ Implementado
- Play/Pause/Siguiente/Anterior
- Detección automática de reproductor activo
- Caché de metadatos
- Soporte múltiples reproductores (Chromium, navegadores)
- **[NUEVO]** Slider de volumen
- **[NUEVO]** Barra de progreso con posición/duración

### ❌ Faltante
- [ ] Mostrar álbum en metadatos
- [ ] Soporte para Spotify (requiere plugin DBus específico)
- [ ] Cola de reproducción / Up Next
- [ ] Búsqueda de canciones en reproductor
- [ ] Shuffle/Repeat (bucles)
- [ ] Historial de reproducción
- [ ] Sincronización con múltiples dispositivos
- [ ] Gapless playback indicator
- [ ] Calidad de audio selector (si el reproductor lo soporta)

---
### Criterios (Música)
- Metadatos: `album`, `artist`, `title`, `artUrl` presentes si el reproductor los expone; caché rellena gaps.
- Control: Play/Pause/Siguiente/Anterior con emisión `music-playing-update` inmediata.
- Rendimiento: latencia perceptible < 200ms en UI.


## 🔊 **AUDIO/VOLUMEN** (Prioridad: ALTA)
### ✅ Implementado
- Control de volumen general (slider)
- Mute/Unmute
- Cambio de dispositivo de salida (parcial)

### ❌ Faltante
- [x] Selector visual de dispositivos de audio activo
- [ ] Control de volumen por aplicación
- [ ] Ecualizador de audio
- [ ] Perfiles de audio (gaming, música, cine)
- [ ] Monitoreo de entrada de micrófono
- [ ] Control de volumen de entrada
- [ ] Soporte para Bluetooth audio
- [ ] Sincronización multi-zona (Pulseaudio/Pipewire)

---
### Criterios (Audio)
- Device selector: lista todos los Sinks de PipeWire y permite set-default; refleja cambios en tiempo real.
- Volumen: cambios reflejados en UI y evento `volume-changed`.
- Persistencia: último dispositivo preferido recordado.


## 🔌 **RED/WIFI** (Prioridad: ALTA)
### ✅ Implementado
- Mostrar redes Wi-Fi disponibles
- Conectar/desconectar WiFi
- Mostrar conexión activa

### ❌ Faltante
- [ ] VPN integrado/gestor de VPN
- [ ] Hotspot/Tethering (compartir internet)
- [ ] Proxy configuration
- [ ] DNS personalizado UI
- [ ] Monitor de velocidad de red
- [ ] Estadísticas de consumo de datos
- [ ] IPv6 support visible
- [ ] Saved networks manager (editar contraseñas)
- [ ] Network troubleshooting tools
- [ ] Auto-connect improvements

---
### Criterios (Red)
- Estado conexión: SSID, calidad de señal y velocidad.
- VPN: perfiles listados, conectar/desconectar con feedback.
- Troubleshooting: ping a gateway/DNS y reporte simple.


## 📱 **BLUETOOTH** (Prioridad: MEDIA)
### ✅ Implementado
- Listar dispositivos Bluetooth
- Emparejar/desemparejar
- Conectar/desconectar

### ❌ Faltante
- [ ] Indicador de batería de dispositivos conectados
- [ ] Audio sink switching (cambiar entre parlantes/headsets)
- [ ] File transfer (Bluetooth OBEX)
- [ ] Input devices (mouse/keyboard) management
- [ ] Bluetooth adapter selector
- [ ] Discovery timeout improvements
- [ ] Device profiles visualization
- [ ] Connection history

---

## ⚡ **ENERGÍA/BATERÍA** (Prioridad: MEDIA)
### ✅ Implementado
- Mostrar nivel de batería
- Mostrar tiempo restante
- Mostrar estado de carga

### ❌ Faltante
- [ ] Perfiles de energía (rendimiento/equilibrado/ahorro)
- [ ] Planificador de apagado automático
- [ ] Hibernación (si hardware lo soporta)
- [ ] Ajustes de pantalla por batería
- [ ] Monitor de consumo por aplicación
- [ ] Notificaciones de batería baja mejoradas
- [ ] Thermal management UI
- [ ] CPU frequency scaling visualization
- [ ] Battery health indicator

---

## 🌓 **TEMAS/APARIENCIA** (Prioridad: MEDIA)
### ✅ Implementado
- Toggle light/dark theme
- Icono dinámico

### ❌ Faltante
- [ ] Selector de colores primarios
- [ ] Selector de tipografía
- [ ] Custom wallpaper selector
- [ ] Efectos visuales (blur, transparency level)
- [ ] Animation settings
- [ ] Desktop icons layout
- [ ] Cursor theme selector
- [ ] Font size scaling
- [ ] Color blind friendly presets

---

## 🖥️ **DESKTOPS VIRTUALES** (Prioridad: MEDIA)
### ✅ Implementado
- Crear/eliminar desktops
- Cambiar entre desktops
- Mostrar preview de desktops

### ❌ Faltante
- [ ] Renombrar desktops
- [ ] Drag & drop entre desktops
- [ ] Desktop-specific wallpapers
- [ ] Hot-keys para cambiar desktops mejorados
- [ ] Show all windows across desktops (búsqueda)
- [ ] Default apps per desktop
- [ ] Persistent desktop layouts

---

## 🪟 **GESTOR DE VENTANAS** (Prioridad: ALTA)
### ✅ Implementado
- Cambiar entre ventanas
- Minimizar/maximizar
- Cerrar ventanas
- Window snapping básico

### ❌ Faltante
- [ ] Tile layout automático
- [ ] Window groups
- [ ] Expose/Alt-Tab mejorado con vista previa
- [ ] Always on top setting
- [ ] Window opacity control
- [ ] Maximize vertical/horizontal
- [ ] Window move/resize snap points
- [ ] Workspace-aware window placement
- [ ] Window decoration theme
- [ ] Fullscreen mode improvements

---
### Criterios (Ventanas)
- Atajos básicos: split izquierda/derecha, cambiar foco, mover ventana.
- Indicador visual discreto del layout activo.


## 📂 **ARCHIVO/GESTOR DE ARCHIVOS** (Prioridad: MEDIA)
### ✅ Implementado
- Menú de aplicaciones rápido
- Buscar aplicaciones

### ❌ Faltante
- [ ] File manager sidebar integration
- [ ] Recent files widget
- [ ] Folders shortcuts
- [ ] Drag & drop desktop support
- [ ] Desktop right-click menu
- [ ] Create file/folder shortcuts
- [ ] Mount points visualization
- [ ] Storage usage indicator
- [ ] Trash/recycle bin indicator

---

## 🎮 **GAMING/PERFORMANCE** (Prioridad: BAJA)
### ✅ Implementado
- None

### ❌ Faltante
- [ ] Game mode (disable notifications, lock fps, etc.)
- [ ] Performance monitor
- [ ] GPU usage indicator
- [ ] RAM usage widget
- [ ] CPU temperature widget
- [ ] Disk I/O monitor
- [ ] Process killer quick access
- [ ] OBS integration
- [ ] Gamepad input visualization

---

## 🔐 **SEGURIDAD/PRIVACIDAD** (Prioridad: MEDIA)
### ✅ Implementado
- None

### ❌ Faltante
- [ ] Firewall toggle
- [ ] Privacy settings panel
- [ ] Permissions manager
- [ ] Camera/microphone access indicator
- [ ] Location services toggle
- [ ] Tracker blocking settings
- [ ] VPN status indicator
- [ ] Encryption status
- [ ] Screen lock timeout
- [ ] Session lock quick access

---

## 🌐 **SISTEMA/INFORMACIÓN** (Prioridad: BAJA)
### ✅ Implementado
- Display server detection
- Logout/shutdown/reboot

### ❌ Faltante
- [ ] System information panel (CPU, GPU, RAM)
- [ ] Kernel version display
- [ ] Uptime widget
- [ ] Network interfaces info
- [ ] System updates checker
- [ ] Changelog viewer
- [ ] System health monitor
- [ ] Locale/timezone settings
- [ ] Time sync status

---

## ⏰ **RELOJ/CALENDARIO** (Prioridad: BAJA)
### ✅ Implementado
- Reloj en panel
- Reloj en desktop (widget)

### ❌ Faltante
- [ ] Integración de calendario
- [ ] Eventos próximos widget
- [ ] Timezone selector
- [ ] Multiple clocks (world time)
- [ ] Alarm/reminder system
- [ ] Pomodoro timer
- [ ] Stopwatch widget
- [ ] Countdown timer
- [ ] Date format selector

---

## 🔍 **BÚSQUEDA GLOBAL** (Prioridad: ALTA)
### ✅ Implementado
- SearchMenuComponent (búsqueda básica)

### ❌ Faltante
- [ ] Búsqueda de archivos en sistema
- [ ] Búsqueda en settings
- [ ] Búsqueda de aplicaciones mejorada
- [ ] Búsqueda en historial
- [ ] Búsqueda web integrada
- [ ] Búsqueda de emojis
- [ ] Busqueda por comando/acciones
- [ ] Indexación en background
- [ ] Filtros de búsqueda avanzados

---
### Criterios (Búsqueda)
- Indexación incremental y en background.
- Resultados con paginación y filtros por tipo.


## ⚙️ **CONFIGURACIÓN** (Prioridad: ALTA)
### ✅ Implementado
- Panel básico de configuración
- Cambios de tema/brillo
- [NUEVO] Configuración del Sistema Operativo (UI)
  - Border radius (1-20px)
  - Color primario y color de énfasis
  - Toggle dark mode / light mode
  - Selector de tema GTK
  - Selector de cursor
  - Selector de pack de iconos
  - Persistencia via tauri-plugin-config-manager

### ❌ Faltante
- [ ] Gestor de startup applications
- [ ] Gestor de extensiones/plugins
- [x] Configuración de atajos de teclado
- [ ] Configuración de ratón/touchpad
- [ ] Configuración de pantalla (resolución, refresh rate)
- [ ] Gestor de idiomas
- [ ] Configuración de tiempo/fecha
- [ ] Copia de configuración (backup/restore)
- [ ] Reset to defaults
- [ ] Import/export settings

---
### Criterios (Config)
- Edición de atajos con validación y persistencia.
- Backup/restore simple (JSON) y valores por defecto.

## 📱 **NOTIFICACIONES** (Prioridad: MEDIA)
### ✅ Implementado
- Centro de notificaciones
- Historial de notificaciones
- Borrar notificaciones

### ❌ Faltante
- [ ] Grupos de notificaciones por app
- [ ] Configuración per-app (permitir/bloquear)
- [ ] Sonidos personalizados
- [ ] Snooze/Pause notifications
- [ ] Do not disturb mode mejorado
- [ ] Quiet hours configuration
- [ ] Notification actions execution
- [ ] Rich notifications (buttons, inputs)
- [ ] Desktop notifications independent settings

---

## 🎤 **MICRÓFONO/INPUT** (Prioridad: BAJA)
### ✅ Implementado
- None

### ❌ Faltante
- [ ] Input device selector
- [ ] Mic level indicator
- [ ] Noise suppression toggle
- [ ] Echo cancellation settings
- [ ] Input device configuration panel
- [ ] Recording application access indicator
- [ ] Default microphone selector

---

## 🖱️ **RATÓN/TOUCHPAD** (Prioridad: BAJA)
### ✅ Implementado
- None

### ❌ Faltante
- [ ] Sensitivity settings
- [ ] Acceleration configuration
- [ ] Touchpad gesture settings
- [ ] Button customization
- [ ] Scroll direction (natural/reverse)
- [ ] Middle click simulation
- [ ] Pointer trails
- [ ] Click sound
- [ ] Tap to click

---

## 📴 **POWER MANAGEMENT** (Prioridad: MEDIA)
### ✅ Implementado
- Logout/Shutdown/Reboot/Suspend

### ❌ Faltante
- [ ] Hibernation
- [ ] Sleep mode (deep sleep)
- [ ] Wake-on-LAN
- [ ] Schedule shutdown
- [ ] Inactivity timeout
- [ ] Lid close behavior
- [ ] Power button behavior customization
- [ ] Auto-brightness
- [ ] Screen timeout manager

---

## 📡 **CONECTIVIDAD** (Prioridad: MEDIA)
### ✅ Implementado
- WiFi toggle
- Bluetooth toggle

### ❌ Faltante
- [ ] Airplane mode
- [ ] Ethernet connection UI
- [ ] Mobile hotspot indicator
- [ ] Connection history
- [ ] Network diagnostics tools
- [ ] Signal strength visualization
- [ ] Auto-switch networks

---

## 🌍 **INTEGRACIONES** (Prioridad: BAJA)
### ✅ Implementado
- None

### ❌ Faltante
- [ ] Google Drive sync
- [ ] Nextcloud integration
- [ ] Cloud storage shortcuts
- [ ] Mail client quick access
- [ ] Social media integration
- [ ] WebDAV support
- [ ] FTP file access
- [ ] SSH quick connect

---

## 📊 **ESTADÍSTICAS** (Prioridad: BAJA)
### ✅ Implementado
- None

### ❌ Faltante
- [ ] Weekly activity report
- [ ] App usage statistics
- [ ] Screen time tracker
- [ ] Energy consumption graph
- [ ] Network usage history
- [ ] Storage usage breakdown
- [ ] Performance metrics dashboard

---

## 🔄 **SINCRONIZACIÓN** (Prioridad: BAJA)
### ✅ Implementado
- None

### ❌ Faltante
- [ ] Settings sync
- [ ] Cross-device clipboard
- [ ] File sync
- [ ] Bookmark sync
- [ ] Password manager integration
- [ ] Calendar sync
- [ ] Contact sync
- [ ] Note sync

---

## 🎨 **PERSONALIZACIÓN AVANZADA** (Prioridad: BAJA)
### ✅ Implementado
- Theme toggle (light/dark)

### ❌ Faltante
- [ ] Custom CSS injection
- [ ] Plugin system
- [ ] Custom widgets creation
- [ ] Script automation
- [ ] Macro system
- [ ] Custom keybindings
- [ ] Context menu customization
- [ ] Desktop effects/animaciones
- [ ] Custom panel layouts

---

## 📈 **ROADMAP SUGERIDO**

### Fase 1: Estabilidad Base (CRÍTICO)
1. ✅ Volumen + progreso música
2. ⏳ Timeouts y reintentos en D-Bus
3. ⏳ Manejo graceful de desconexiones
4. ⏳ Validación de estado pre-comandos
5. Notificaciones de errores al usuario
6. Settings persistentes

### Fase 2: Experiencia Completa (IMPORTANTE)
1. Tiling window manager
2. Búsqueda global mejorada
3. Control center avanzado
4. Gestor de aplicaciones al startup
5. Configuración de atajos
6. Multi-monitor soporte

### Fase 3: Características Premium (NICE-TO-HAVE)
1. Performance monitor
2. Gaming mode
3. Extensiones/plugins
4. Sincronización settings
5. Cloud integration
6. Advanced power management

---

## 📝 **NOTAS TÉCNICAS**

### Dependencias Actuales
- Tauri 2.0
- Vue 3
- Vite
- zbus (D-Bus)
- Wayland/X11 support

### Mejoras Técnicas Necesarias
- [ ] Error handling globalizado
- [ ] Retry mechanism genérico
- [ ] Connection pooling D-Bus
- [ ] Performance profiling
- [ ] Logging estructurado
- [ ] Testing framework
- [ ] CI/CD pipeline
- [ ] Documentation autogenerada
- [ ] API versioning

---

**Última actualización:** 26 de Diciembre 2025
**Estado general:** ~35% completo (funcionalidades principales)
**Estabilidad:** Buena para uso básico, mejorable para producción
