# Mapeo de Atajos de Teclado VasakOS → xbindkeys

Este documento muestra cómo se convierten los atajos de teclado de VasakOS al formato que entiende `xbindkeys`.

## Formato de Conversión

### Modificadores
| VasakOS       | xbindkeys  |
|---------------|------------|
| `Ctrl`        | `Control`  |
| `Alt`         | `Alt`      |
| `Shift`       | `Shift`    |
| `Super`       | `Mod4`     |

### Teclas Especiales
| VasakOS       | xbindkeys  |
|---------------|------------|
| `Space`       | `space`    |
| `Enter`       | `Return`   |
| `Print`       | `Print`    |
| `Escape`      | `Escape`   |
| Letras (A-Z)  | minúsculas |

## Ejemplos de Atajos VasakOS

### Atajos del Sistema

**Terminal (Ctrl+Alt+T)**
```
VasakOS:   Ctrl+Alt+T
xbindkeys: Control+Alt + t
Acción:    gnome-terminal (u otra terminal)
```

**Gestor de Archivos (Super+E)**
```
VasakOS:   Super+E
xbindkeys: Mod4 + e
Acción:    nautilus (u otro gestor)
```

**Bloquear Pantalla (Super+L)**
```
VasakOS:   Super+L
xbindkeys: Mod4 + l
Acción:    loginctl lock-session
```

**Captura de Pantalla (Print)**
```
VasakOS:   Print
xbindkeys: Print
Acción:    gnome-screenshot
```

### Atajos de VasakOS

**Búsqueda Global (Super+Space)**
```
VasakOS:   Super+Space
xbindkeys: Mod4 + space
Acción:    dbus-send ... OpenSearch
```

**Menú de Aplicaciones (Super)**
```
VasakOS:   Super
xbindkeys: Mod4
Acción:    dbus-send ... OpenMenu
```

**Centro de Control (Super+C)**
```
VasakOS:   Super+C
xbindkeys: Mod4 + c
Acción:    dbus-send ... OpenControlCenter
```

**Configuración (Super+,)**
```
VasakOS:   Super+,
xbindkeys: Mod4 + ,
Acción:    dbus-send ... OpenConfigApp
```

## Archivo de Configuración Generado

El sistema genera automáticamente `~/.xbindkeysrc` con este formato:

```bash
# Auto-generated xbindkeys configuration
# WARNING: This file is managed by VasakOS. Manual edits may be overwritten.

# Shortcut: Ctrl+Alt+T -> gnome-terminal
"sh -c 'gnome-terminal'"
  Control+Alt + t

# Shortcut: Super+Space -> dbus-send ... OpenSearch
"sh -c 'dbus-send --session --dest=org.vasak.os.Desktop --type=method_call / org.vasak.os.Desktop.OpenSearch'"
  Mod4 + space

# ... más atajos ...
```

## Notas Técnicas

1. **Separador**: xbindkeys usa ` + ` (espacio-más-espacio) entre modificadores y la tecla
2. **Modificadores múltiples**: Se concatenan sin espacios: `Control+Alt + t`
3. **Mayúsculas/Minúsculas**: Las letras siempre en minúsculas para la tecla final
4. **Teclas especiales**: Tienen nombres específicos (ej: `space`, `Return`)
5. **Comando**: Se envuelve en `sh -c '...'` para compatibilidad con shell

## Limitaciones

- **X11 únicamente**: `xbindkeys` solo funciona en X11, no en Wayland
- **Para Wayland**: Se usa D-Bus/compositor específico (implementación parcial)
- **Backup manual**: El archivo `~/.xbindkeysrc` se sobrescribe completamente

## Verificación

Para ver los atajos actualmente registrados:

```bash
cat ~/.xbindkeysrc
```

Para recargar atajos manualmente:

```bash
pkill xbindkeys && xbindkeys
```

Para probar un atajo D-Bus directamente:

```bash
dbus-send --session --dest=org.vasak.os.Desktop --type=method_call / org.vasak.os.Desktop.OpenMenu
```
