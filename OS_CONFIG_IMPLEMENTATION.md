# Configuración del Sistema Operativo - Implementación Técnica

## Resumen Ejecutivo

Se ha implementado una sección completa de configuración del sistema operativo en VasakOS que permite a los usuarios personalizar la apariencia visual y el comportamiento del escritorio de forma sencilla e intuitiva.

**Fecha**: 27 de Diciembre de 2025  
**Estado**: ✅ Completado y compilando  
**Ubicación**: `/apps/configuration/system` en la app de configuración

---

## Características Implementadas

### 1. Configuración Visual (Apariencia)

#### Border Radius
- **Rango**: 1 - 20 px
- **Tipo**: Slider interactivo
- **Valor por defecto**: 8px
- **Persistencia**: Archivo JSON local
- **Aplicación**: CSS variable `--border-radius` en el documento

#### Color Primario
- **Selector**: Color picker + input de texto (formato hexadecimal)
- **Valor por defecto**: #0084FF (azul VasakOS)
- **Persistencia**: Archivo JSON local
- **Aplicación**: CSS variable `--primary-color`

#### Color de Énfasis
- **Selector**: Color picker + input de texto
- **Valor por defecto**: #FF6B6B (rojo)
- **Persistencia**: Archivo JSON local
- **Aplicación**: CSS variable `--accent-color`

#### Dark Mode / Light Mode
- **Tipo**: Toggle switch
- **Integración**: gsettings (org.gnome.desktop.interface / color-scheme)
- **Valor por defecto**: Light mode (prefer-light)
- **Aplicación**: Clase `.dark-mode` en el HTML + gsettings

### 2. Configuración de Sistema

#### Tema GTK
- **Tipo**: Selector desplegable dinámico
- **Fuentes**: 
  - `/usr/share/themes` (temas del sistema)
- **Integración**: gsettings (org.gnome.desktop.interface / gtk-theme)
- **Valor por defecto**: Adwaita
- **Comportamiento**: Auto-detecta `-dark` suffix cuando dark mode está activo

#### Cursor
- **Tipo**: Selector desplegable dinámico
- **Fuentes**:
  - `/usr/share/icons` (cursores del sistema)
  - `~/.local/share/icons` (cursores locales del usuario)
- **Integración**: gsettings (org.gnome.desktop.interface / cursor-theme)
- **Valor por defecto**: Adwaita
- **Carga dinámica**: Detecta cursores disponibles en tiempo de ejecución

#### Pack de Iconos
- **Tipo**: Selector desplegable dinámico
- **Fuentes**:
  - `/usr/share/icons` (packs del sistema)
  - `~/.local/share/icons` (packs locales)
- **Criterio**: Solo detecta packs que tengan `index.theme`
- **Valor por defecto**: Adwaita
- **Nota**: Requiere refrescar las aplicaciones de Tauri para actualizar

---

## Arquitectura Técnica

### Backend (Rust)

**Archivo**: `src-tauri/src/commands/system_config.rs` (~220 líneas)

#### Estructura Principal

```rust
pub struct SystemConfig {
    pub border_radius: u32,
    pub primary_color: String,
    pub accent_color: String,
    pub dark_mode: bool,
    pub icon_pack: String,
    pub cursor_theme: String,
    pub gtk_theme: String,
}
```

#### Funciones Principales

1. **`get_system_config()`** `[async]`
   - Lee la configuración desde `~/.config/vasak/system_config.json`
   - Retorna valores por defecto si el archivo no existe
   - Manejo de errores con mensajes claros

2. **`set_system_config(config)`** `[async]`
   - Aplica cambios al sistema (gsettings, CSS vars)
   - Persiste configuración en archivo JSON
   - Crea directorio si no existe
   - Retorna la configuración guardada

3. **`apply_system_config(config)`** `[async]`
   - Orquesta cambios al sistema:
     - `set_gtk_theme()`: Configura tema GTK
     - `set_cursor_theme()`: Configura cursor
     - `set_dark_mode()`: Configura preferencia de color

4. **`get_gtk_themes()`** `[async]`
   - Lista temas disponibles en `/usr/share/themes`
   - Retorna vec ordenado alfabéticamente
   - Fallback a `["Adwaita"]` si no hay temas

5. **`get_cursor_themes()`** `[async]`
   - Explora `/usr/share/icons` y `~/.local/share/icons`
   - Detecta directorios válidos
   - Retorna set deduplicado y ordenado

6. **`get_icon_packs()`** `[async]`
   - Explora rutas de iconos
   - Valida presencia de `index.theme`
   - Retorna packs disponibles

#### Persistencia

- **Ubicación**: `~/.config/vasak/system_config.json`
- **Formato**: JSON legible (pretty-printed)
- **Estructura**:
  ```json
  {
    "border_radius": 8,
    "primary_color": "#0084FF",
    "accent_color": "#FF6B6B",
    "dark_mode": false,
    "icon_pack": "Adwaita",
    "cursor_theme": "Adwaita",
    "gtk_theme": "Adwaita"
  }
  ```

#### Integración del Sistema

- **gsettings**: Cambios de color-scheme, gtk-theme, cursor-theme
- **CSS Variables**: Inyectadas en `document.documentElement.style`
- **Classes HTML**: Clase `.dark-mode` para estilos condicionales

### Frontend (Vue 3)

**Archivo**: `src/views/apps/configuration/ConfigOSView.vue` (~620 líneas)

#### Componentes Principales

1. **`<template>` - Estructura HTML**
   - Layout con `ConfigAppLayout` (navbar + sidebar + contenido)
   - Spinner de carga inicial
   - Alertas de error y éxito
   - Formulario organizado en secciones

2. **Secciones de Configuración**
   - **🎨 Apariencia**: Border radius, colores, dark mode
   - **🖥️ Tema GTK**: Selector desplegable
   - **🖱️ Cursor**: Selector desplegable
   - **🎯 Iconos**: Selector desplegable + advertencia

3. **Gestión de Estado**
   - `config`: ref con estructura SystemConfig
   - `gtkThemes`, `cursorThemes`, `iconPacks`: refs para opciones
   - `loading`, `saving`: refs para UI de carga
   - `error`, `successMessage`: refs para retroalimentación

4. **Métodos Principales**

   - **`onMounted()`**
     - Carga configuración actual: `get_system_config()`
     - Carga opciones disponibles en paralelo
     - Manejo de errores

   - **`saveConfig()`**
     - Valida border radius (1-20)
     - Invoca `set_system_config(config)`
     - Aplica CSS vars con `applyThemeToDOM()`
     - Muestra mensaje de éxito temporalmente

   - **`applyThemeToDOM()`**
     - Inyecta CSS variables en document
     - Gestiona clase `.dark-mode`

   - **`resetToDefaults()`**
     - Confirma con el usuario
     - Restaura valores por defecto
     - Guarda automáticamente

#### Validación

```typescript
const isFormValid = computed(() => {
  return (
    config.value.border_radius >= 1 &&
    config.value.border_radius <= 20 &&
    config.value.primary_color &&
    config.value.accent_color &&
    config.value.gtk_theme &&
    config.value.cursor_theme &&
    config.value.icon_pack
  );
});
```

#### Estilos

- **CSS Variables**: `--text-primary`, `--surface-2`, `--primary-color`, `--accent-color`
- **Componentes Estilizados**:
  - Slider con thumb personalizado
  - Color picker + input texto
  - Selects mejorados
  - Toggle switch animado
  - Botones con estados (hover, disabled)

---

## Flujo de Datos

```
┌─────────────────────┐
│  ConfigOSView.vue   │  (Frontend Vue 3)
└──────────┬──────────┘
           │
      onMounted() / saveConfig()
           │
           ▼
┌──────────────────────────────────┐
│  Tauri Invoke Commands           │
│  - get_system_config             │
│  - set_system_config             │
│  - get_gtk_themes                │
│  - get_cursor_themes             │
│  - get_icon_packs                │
└──────────┬───────────────────────┘
           │
           ▼
┌──────────────────────────────────┐
│  Backend Rust (system_config.rs) │
└──────────┬───────────────────────┘
           │
    ┌──────┼──────┐
    │      │      │
    ▼      ▼      ▼
 JSON   gsettings CSS Vars
 File   (Kernel)  (DOM)
```

---

## Integración en Rutas

**Archivo**: `src/routes/index.ts`

```typescript
{
  path: "system",
  component: () => import("@/views/apps/configuration/ConfigOSView.vue"),
}
```

**Acceso**: `/#/apps/configuration/system`

---

## Sidebar Navigation

**Archivo**: `src/components/areas/configuration/ConfigSidebarArea.vue`

Se agregó botón con icono `settings` que navega a `/apps/configuration/system`.

Orden de items:
1. Information (info)
2. Style Settings (style)
3. **OS Settings** (system) ← NUEVO
4. Audio Settings (audio)
5. Keyboard Shortcuts (shortcuts)
6. Network Settings (network)
7. Bluetooth Settings (bluetooth)

---

## Compilación y Estado

### Backend Rust
✅ **Compilación exitosa**
- Comando: `cargo check`
- Resultado: 23 warnings (pre-existentes, no relacionados a system_config.rs)
- Errores: 0

### Frontend Vue
✅ **Compilación exitosa**
- Comando: `npm run build`
- Resultado: `✓ built in 2.19s`
- Tamaño ConfigOSView.js: 6.64 kB (gzip: 2.35 kB)
- Errores TypeScript: 0

---

## Próximos Pasos Opcionalmente

1. **Refrescar Apps de Tauri**: Implementar comando que notifique a todas las ventanas de Tauri cuando cambie el icon pack (reload de assets)

2. **Sincronización de Configuración**: Agregar export/import de configuración (backup/restore)

3. **Historial de Cambios**: Registrar cambios en un log para auditoría

4. **Validación de Temas**: Verificar validez de temas antes de aplicarlos

5. **Undo/Redo**: Implementar deshacer/rehacer cambios

---

## Testing Manual

### Prueba 1: Cargar Configuración
- ✅ Abrir app de Configuración → pestaña Sistema
- ✅ Cargar configuración del archivo (o valores por defecto)
- ✅ Mostrar opciones disponibles

### Prueba 2: Cambiar Border Radius
- ✅ Mover slider a 15
- ✅ Ver preview en tiempo real (no requiere guardar)
- ✅ Guardar cambios
- ✅ Verificar persistencia en `~/.config/vasak/system_config.json`

### Prueba 3: Cambiar Colores
- ✅ Seleccionar color primario con color picker
- ✅ Cambiar color de énfasis manualmente
- ✅ Guardar y ver aplicación en UI

### Prueba 4: Dark Mode
- ✅ Toggle dark mode
- ✅ Verificar llamada a gsettings
- ✅ Verificar clase `.dark-mode` en HTML

### Prueba 5: Temas GTK
- ✅ Selector carga temas disponibles
- ✅ Seleccionar tema diferente
- ✅ Guardar y verificar cambio en sistema

### Prueba 6: Reset to Defaults
- ✅ Modificar varias opciones
- ✅ Clic en "Restablecer Valores por Defecto"
- ✅ Confirmar en diálogo
- ✅ Valores vuelven a defecto

---

## Notas Técnicas

- **Async/Await**: Todos los comandos Rust son `async` para no bloquear UI
- **Error Handling**: Mensajes de error claros y específicos en ambos lados
- **Validación**: Validación en frontend (UI) y backend (aplicación)
- **Compatibilidad**: Funciona con Linux GNOME (gsettings)
- **Rendimiento**: Carga dinámica de temas/cursores sin bloquear UI

---

## Archivos Modificados

1. ✅ `src-tauri/src/commands/system_config.rs` (creado)
2. ✅ `src-tauri/src/commands/mod.rs` (actualizado con módulo + exports)
3. ✅ `src-tauri/src/lib.rs` (agregados comandos al invoke_handler)
4. ✅ `src/views/apps/configuration/ConfigOSView.vue` (creado)
5. ✅ `src/views/apps/configuration/ConfigStyleView.vue` (actualizado)
6. ✅ `src/components/areas/configuration/ConfigSidebarArea.vue` (agregado botón)
7. ✅ `src/routes/index.ts` (agregada ruta)
8. ✅ `FUNCIONALIDADES_FALTANTES.md` (actualizado estado)
9. ✅ `OS_CONFIG_IMPLEMENTATION.md` (este documento)

---

**Implementación completada con éxito** ✅
