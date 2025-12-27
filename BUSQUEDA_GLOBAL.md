# Búsqueda Global - Implementación Completa

## ✅ Resumen de Cambios

La búsqueda global se ha implementado como una **ventana separada** (no integrada en App.vue), similar a cómo funcionan los applets. Es callable desde:
1. **Botón en el sidebar de configuración** (interfaz gráfica)
2. **Comando Tauri** `toggle_search`
3. **Método D-Bus** `org.vasak.os.Desktop.OpenSearch` (para shortcuts del sistema)

---

## 🔧 Backend (Rust)

### Módulo de búsqueda: `src-tauri/src/search.rs`
- **Parser de .desktop files**: Escanea `/usr/share/applications`, `/usr/local/share/applications`, `~/.local/share/applications`
- **Caché inteligente**: Refresco automático cada 5 minutos
- **Fuzzy matching**: Sistema de puntuación de coincidencias
  - 100: coincidencia exacta
  - 90: comienza con query
  - 70: contiene query
  - 50: fuzzy match (caracteres en orden)
- **Acciones del sistema**: apagar, reiniciar, suspender, bloquear, logout, configuración

#### Funciones principales:
```rust
pub fn search(query: &str, limit: usize) -> Vec<SearchResult>
pub fn search_applications(query: &str, limit: usize) -> Vec<SearchResult>
pub fn get_system_actions(query: &str) -> Vec<SearchResult>
```

### Ventana de búsqueda: `src-tauri/src/windows_apps/search.rs`
- Crea una ventana decorless con propiedades GTK
- Posicionada al centro de la pantalla
- Tamaño: 700x600 (escalable)
- Always on top, skip taskbar

```rust
pub async fn create_search_window(app: AppHandle) -> Result<(), Box<dyn std::error::Error>>
```

### Comandos: `src-tauri/src/commands/search_window.rs`
- **`toggle_search`**: Alterna visibilidad de la ventana o la crea si no existe
- **`global_search`**: Búsqueda con límite de 50-100 resultados
- **`execute_search_result`**: Ejecuta aplicaciones o acciones del sistema

### D-Bus: `src-tauri/src/dbus_service.rs`
Se agregó soporte para el método `OpenSearch` / `ToggleSearch` que puede ser llamado desde sistemas de shortcuts.

---

## 🎨 Frontend (Vue 3)

### Vista de búsqueda: `src/views/SearchView.vue`
- **Modal elegante** con backdrop blur y animaciones suaves
- **Debounce de 150ms** en el input para optimizar búsquedas
- **Navegación por teclado**:
  - `↑/↓`: Navegar resultados
  - `Enter`: Ejecutar resultado seleccionado
  - `Esc`: Cerrar ventana
- **Diseño adaptable** con Catppuccin colors
- **Icono emoji** por categoría (📦 App, 📄 Archivo, ⚡ Acción)
- **Scrollbar personalizado** con tema oscuro

### Integración en rutas: `src/routes/index.ts`
```typescript
{ path: "/search", component: () => import("@/views/SearchView.vue") }
```

### Botón en sidebar: `src/components/areas/configuration/ConfigSidebarArea.vue`
- Nuevo botón "🔍 Global Search" en la parte superior del sidebar
- Al clickear, abre la ventana de búsqueda
- Usa el icono "search" del sistema de vicons

---

## 📦 Integraciones

### Archivo de configuración
```
src-tauri/Cargo.toml
- dirs = "5.0"  (para acceso a directorios XDG)
```

### Cambios a lib.rs
- Registra módulo `search`
- Exporta comandos `toggle_search`, `global_search`, `execute_search_result`
- Registra ventana en el builder de Tauri

### Cambios a App.vue
- Restaurado a su estado original (sin componentes de búsqueda integrados)
- Solo maneja rutas y tema

---

## 🚀 Cómo usar

### 1. Desde la interfaz gráfica
Clickear el botón 🔍 en el sidebar de configuración

### 2. Desde comandos Tauri (JavaScript)
```typescript
import { invoke } from '@tauri-apps/api/core'
await invoke('toggle_search')
```

### 3. Desde D-Bus (para shortcuts del sistema)
```bash
dbus-send --session /org/vasak/os/Desktop \
  org.vasak.os.Desktop.OpenSearch
```

### 4. Definir shortcut global en `~/.config/vasak/shortcuts.json`
```json
{
  "search": {
    "keys": ["Super+Space"],
    "command": "dbus-send --session /org/vasak/os/Desktop org.vasak.os.Desktop.OpenSearch"
  }
}
```

---

## ✨ Características

✅ **Performante**
- Caché con refresco inteligente
- Límite de resultados (50-100)
- Debounce de 150ms
- Búsqueda sin bloqueos (async)

✅ **Estético**
- Diseño moderno con Catppuccin colors
- Animaciones suaves (escala, desvanecimiento)
- Backdrop blur (efecto cristal)
- Gradientes sutiles
- Iconos emoji por categoría

✅ **Accesible**
- Navegación completa por teclado
- Alt+Space (via D-Bus) para abrir
- Atajos para ejecutar acciones del sistema

✅ **Extensible**
- Fácil agregar más categorías de búsqueda
- Estructura modular (search module separado)
- Compatible con archivos recientes y búsqueda de archivos (futuro)

---

## 🔄 Flujo de búsqueda

```
Usuario escribe → Debounce 150ms → invoke('global_search', query)
  ↓
Backend: fuzzy matching en caché de .desktop files
Backend: matching en acciones del sistema
  ↓
Resultados ordenados por score
  ↓
Frontend: renderiza con animaciones
  ↓
Usuario navega con ↑↓ → Usuario presiona Enter
  ↓
invoke('execute_search_result')
  ↓
Ejecuta aplicación o acción
  ↓
Ventana se cierra
```

---

## 📋 Checklist completado

- ✅ Parser de .desktop files con caché
- ✅ Fuzzy matching con puntuación
- ✅ Acciones del sistema (shutdown, reboot, etc)
- ✅ Ventana separada con estética propia
- ✅ Vista SearchView.vue
- ✅ Navegación por teclado
- ✅ Comando toggle_search
- ✅ Método D-Bus OpenSearch
- ✅ Botón en sidebar de configuración
- ✅ Debounce de entrada
- ✅ Animaciones y transiciones
- ✅ Compilación exitosa (solo warnings no críticos)

---

## 🔮 Mejoras futuras

1. **Búsqueda de archivos recientes** (XDG recent files)
2. **Búsqueda en archivos** (con timeout)
3. **Historial de búsquedas** (local storage)
4. **Temas personalizables** para la ventana
5. **Plugins de búsqueda** (extensibilidad)
6. **Búsqueda fuzzy mejorada** con crate fuzzy-matcher
