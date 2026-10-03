//! Lo que el panel muestra del compositor: los espacios de trabajo y la
//! distribución de teclado (vasak-desktop#151).
//!
//! Las dos salen del IPC de Wayfire y se siguen por sus eventos, con una
//! conexión **propia**: la compartida (`get_wayfire_client`) no se suscribe a
//! eventos, y pedirle `events/watch` cambiaría cuándo se refresca la lista de
//! ventanas, que hoy va por sondeo. Un socket aparte no toca nada de eso.
//!
//! Lo que se puede equivocar —leer la grilla, pasar de un número a una celda,
//! sacar el código corto de una distribución— va en funciones puras con sus
//! pruebas; lo demás es pedir y avisar.

use super::Applet;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use std::sync::OnceLock;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::window_manager::wayfire_ipc::{get_wayfire_client, WayfireClient};

/// Más de esto no entra en la píldora sin sacar a las demás del panel. Una
/// grilla más grande muestra los primeros: los demás siguen a mano de los
/// atajos de Wayfire.
pub const MAX_WORKSPACES: u32 = 10;

/// Los eventos de Wayfire que pueden cambiar lo que muestra la píldora.
///
/// - `wset-workspace-changed`: se cambió de espacio;
/// - `output-gain-focus` y `output-wset-changed`: la pantalla con foco es otra,
///   y su grilla o su espacio también;
/// - `keyboard-modifier-state-changed`: el grupo de xkb —la distribución— viaja
///   con los modificadores.
const WATCHED_EVENTS: [&str; 4] = [
    "wset-workspace-changed",
    "output-gain-focus",
    "output-wset-changed",
    "keyboard-modifier-state-changed",
];

// ─── E s p a c i o s   d e   t r a b a j o ──────────────────────────────────

/// Los espacios de la pantalla con foco, como los dibuja la píldora.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkspaceState {
    /// Cuántos se muestran (la grilla entera, hasta [`MAX_WORKSPACES`]).
    pub count: u32,
    /// El actual, desde 0, en el orden de lectura de la grilla.
    pub active: u32,
    /// Cuántas columnas tiene la grilla: con esto se vuelve de un número a una
    /// celda.
    pub columns: u32,
    /// La pantalla a la que se refieren.
    pub output_id: i64,
}

fn as_u32(value: Option<&Value>) -> Option<u32> {
    let value = value?;
    value
        .as_u64()
        .or_else(|| {
            value
                .as_f64()
                .filter(|v| *v >= 0.0)
                .map(|v| v.round() as u64)
        })
        .and_then(|v| u32::try_from(v).ok())
}

/// Lee el `info` de `window-rules/get-focused-output`.
///
/// Wayfire arma los espacios en una grilla (`grid_width` × `grid_height`) y
/// dice en qué celda está (`x`, `y`). La píldora los numera en orden de
/// lectura: fila por fila, de izquierda a derecha, que es como los recorre el
/// atajo de «siguiente espacio».
pub fn state_from_output(info: &Value) -> Option<WorkspaceState> {
    let workspace = info.get("workspace")?;
    let columns = as_u32(workspace.get("grid_width"))?.max(1);
    let rows = as_u32(workspace.get("grid_height"))?.max(1);
    let x = as_u32(workspace.get("x"))?;
    let y = as_u32(workspace.get("y"))?;
    let output_id = info.get("id").and_then(Value::as_i64)?;

    let total = columns.saturating_mul(rows);
    let count = total.min(MAX_WORKSPACES);
    let active = (y.saturating_mul(columns) + x).min(total.saturating_sub(1));

    Some(WorkspaceState {
        count,
        active,
        columns,
        output_id,
    })
}

/// La celda (`x`, `y`) del espacio número `index`, o `None` si no existe.
pub fn cell_of(index: u32, state: &WorkspaceState) -> Option<(u32, u32)> {
    if index >= state.count {
        return None;
    }
    Some((index % state.columns, index / state.columns))
}

async fn focused_output(client: &WayfireClient) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let response = client
        .send_and_wait("window-rules/get-focused-output", Value::Null)
        .await?;
    response
        .get("info")
        .cloned()
        .ok_or_else(|| "Wayfire no dijo qué pantalla tiene el foco".into())
}

async fn workspace_state_with(
    client: &WayfireClient,
) -> Result<Option<WorkspaceState>, Box<dyn Error + Send + Sync>> {
    Ok(state_from_output(&focused_output(client).await?))
}

/// Los espacios de ahora, por la conexión compartida.
pub async fn current_workspaces() -> Result<Option<WorkspaceState>, String> {
    let client = get_wayfire_client()
        .await
        .ok_or("Wayfire IPC no responde")?;
    workspace_state_with(&client)
        .await
        .map_err(|error| error.to_string())
}

/// Pasa al espacio número `index` de la pantalla con foco.
pub async fn switch_to(index: u32) -> Result<(), String> {
    let client = get_wayfire_client()
        .await
        .ok_or("Wayfire IPC no responde")?;
    let state = workspace_state_with(&client)
        .await
        .map_err(|error| error.to_string())?
        .ok_or("Wayfire no dijo cómo es la grilla de espacios")?;
    let (x, y) = cell_of(index, &state).ok_or_else(|| format!("no hay espacio {index}"))?;

    client
        .send_and_wait(
            "vswitch/set-workspace",
            json!({ "x": x, "y": y, "output-id": state.output_id }),
        )
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

// ─── D i s t r i b u c i ó n   d e   t e c l a d o ──────────────────────────

/// La distribución de teclado, como la muestra el panel.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KeyboardLayout {
    /// El código corto que se lee en la píldora: «US», «LA», «ES».
    pub short: String,
    /// El nombre entero, para el globo: «Spanish (Latin American)».
    pub name: String,
    /// La posición de la actual entre las configuradas.
    pub index: usize,
    /// Cuántas hay configuradas. Con una sola no hay a qué cambiar.
    pub count: usize,
}

/// De nombre a código, leído de la lista de xkeyboard-config.
///
/// Wayfire contesta con el nombre largo («Spanish (Latin American)»), que en
/// una píldora no entra. La lista de reglas de xkb (`evdev.lst`) dice qué
/// código le corresponde: en la sección `! layout` cada renglón es
/// `código  Nombre`, y en `! variant` es `variante  código: Nombre`.
pub fn codes_by_name(list: &str) -> HashMap<String, String> {
    let mut codes = HashMap::new();
    let mut section = "";

    for line in list.lines() {
        if let Some(name) = line.strip_prefix("! ") {
            section = name.trim();
            continue;
        }
        let line = line.trim();
        let Some((first, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let rest = rest.trim();
        match section {
            "layout" => {
                codes
                    .entry(rest.to_string())
                    .or_insert_with(|| first.to_string());
            }
            "variant" => {
                if let Some((layout, name)) = rest.split_once(':') {
                    codes
                        .entry(name.trim().to_string())
                        .or_insert_with(|| layout.trim().to_string());
                }
            }
            _ => {}
        }
    }

    codes
}

/// Las dos primeras letras del código, en mayúsculas.
///
/// Sin código —una lista de xkb que no está o un nombre que no figura—, las
/// iniciales del nombre, para que la píldora nunca quede vacía.
pub fn short_label(code: Option<&str>, name: &str) -> String {
    let source: String = match code {
        Some(code) if !code.trim().is_empty() => code.trim().to_string(),
        _ => name
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .filter_map(|word| word.chars().next())
            .collect(),
    };
    source.chars().take(2).collect::<String>().to_uppercase()
}

/// Lee la respuesta de `wayfire/get-keyboard-state`.
pub fn layout_from_state(state: &Value, codes: &HashMap<String, String>) -> Option<KeyboardLayout> {
    let name = state.get("layout").and_then(Value::as_str)?.to_string();
    let count = state
        .get("possible-layouts")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(1)
        .max(1);
    let index = state
        .get("layout-index")
        .and_then(Value::as_u64)
        .map(|index| index as usize)
        .unwrap_or(0)
        .min(count - 1);
    let short = short_label(codes.get(&name).map(String::as_str), &name);

    Some(KeyboardLayout {
        short,
        name,
        index,
        count,
    })
}

/// La lista de xkb, leída una vez: no cambia mientras dura la sesión.
fn xkb_codes() -> &'static HashMap<String, String> {
    static CODES: OnceLock<HashMap<String, String>> = OnceLock::new();
    CODES.get_or_init(|| {
        [
            "/usr/share/X11/xkb/rules/evdev.lst",
            "/usr/share/xkeyboard-config-2/rules/evdev.lst",
        ]
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .map(|list| codes_by_name(&list))
        .unwrap_or_default()
    })
}

async fn keyboard_layout_with(
    client: &WayfireClient,
) -> Result<Option<KeyboardLayout>, Box<dyn Error + Send + Sync>> {
    let state = client
        .send_and_wait("wayfire/get-keyboard-state", Value::Null)
        .await?;
    Ok(layout_from_state(&state, xkb_codes()))
}

/// La distribución de ahora, por la conexión compartida.
pub async fn current_keyboard_layout() -> Result<Option<KeyboardLayout>, String> {
    let client = get_wayfire_client()
        .await
        .ok_or("Wayfire IPC no responde")?;
    keyboard_layout_with(&client)
        .await
        .map_err(|error| error.to_string())
}

/// La siguiente de las configuradas, dando la vuelta.
pub fn next_index(layout: &KeyboardLayout) -> usize {
    (layout.index + 1) % layout.count.max(1)
}

/// Pasa a la distribución siguiente. Con una sola no hace nada.
pub async fn cycle_keyboard_layout() -> Result<Option<KeyboardLayout>, String> {
    let client = get_wayfire_client()
        .await
        .ok_or("Wayfire IPC no responde")?;
    let Some(layout) = keyboard_layout_with(&client)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    if layout.count < 2 {
        return Ok(Some(layout));
    }

    client
        .send_and_wait(
            "wayfire/set-keyboard-state",
            json!({ "layout-index": next_index(&layout) }),
        )
        .await
        .map_err(|error| error.to_string())?;

    keyboard_layout_with(&client)
        .await
        .map_err(|error| error.to_string())
}

// ─── E l   a p p l e t ──────────────────────────────────────────────────────

/// Sigue los eventos de Wayfire y avisa al panel con `workspaces-changed` y
/// `keyboard-layout-changed`, sólo cuando cambió algo.
pub struct CompositorApplet;

/// Una vuelta de seguimiento: conectarse, suscribirse y avisar hasta que el
/// socket se caiga.
async fn follow(app: &AppHandle) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = WayfireClient::connect().await?;
    let mut events = client.subscribe();
    client
        .send_and_wait(
            "window-rules/events/watch",
            json!({ "events": WATCHED_EVENTS }),
        )
        .await?;

    let mut workspaces = workspace_state_with(&client).await.ok().flatten();
    let mut layout = keyboard_layout_with(&client).await.ok().flatten();
    let _ = app.emit("workspaces-changed", &workspaces);
    let _ = app.emit("keyboard-layout-changed", &layout);

    loop {
        let event = match events.recv().await {
            Ok(event) => event,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(_) => return Err("el socket de Wayfire se cerró".into()),
        };
        let name = event
            .get("event")
            .and_then(Value::as_str)
            .unwrap_or_default();

        if name == "keyboard-modifier-state-changed" {
            let now = keyboard_layout_with(&client).await.ok().flatten();
            if now != layout {
                layout = now;
                let _ = app.emit("keyboard-layout-changed", &layout);
            }
        } else {
            let now = workspace_state_with(&client).await.ok().flatten();
            if now != workspaces {
                workspaces = now;
                let _ = app.emit("workspaces-changed", &workspaces);
            }
        }
    }
}

#[async_trait]
impl Applet for CompositorApplet {
    fn name(&self) -> &'static str {
        "compositor"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        tauri::async_runtime::spawn(async move {
            let mut delay = Duration::from_secs(2);
            loop {
                match follow(&app).await {
                    Ok(()) => delay = Duration::from_secs(2),
                    Err(error) => {
                        log::debug!("[compositor] sin eventos de Wayfire: {error}");
                    }
                }
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(30));
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(x: u32, y: u32, w: u32, h: u32) -> Value {
        json!({
            "id": 1,
            "name": "eDP-1",
            "workspace": { "x": x, "y": y, "grid_width": w, "grid_height": h }
        })
    }

    #[test]
    fn una_grilla_de_tres_por_dos_son_seis_espacios() {
        // Lo que manda el Wayfire de esta máquina: 3 × 2, en la celda 0,0.
        let state = state_from_output(&output(0, 0, 3, 2)).expect("grilla");
        assert_eq!(
            state,
            WorkspaceState {
                count: 6,
                active: 0,
                columns: 3,
                output_id: 1
            }
        );
    }

    #[test]
    fn el_actual_se_numera_fila_por_fila() {
        // Segunda fila, tercera columna de una 3 × 2: el sexto, índice 5.
        assert_eq!(state_from_output(&output(2, 1, 3, 2)).unwrap().active, 5);
        assert_eq!(state_from_output(&output(1, 0, 3, 2)).unwrap().active, 1);
    }

    #[test]
    fn una_grilla_enorme_se_corta_en_el_maximo() {
        let state = state_from_output(&output(0, 0, 5, 5)).unwrap();
        assert_eq!(state.count, MAX_WORKSPACES);
    }

    #[test]
    fn las_coordenadas_con_coma_tambien_valen() {
        // Wayfire 0.11 pasó la geometría a flotante; la grilla podría seguir.
        let info = json!({
            "id": 3,
            "workspace": { "x": 1.0, "y": 0.0, "grid_width": 2.0, "grid_height": 2.0 }
        });
        assert_eq!(state_from_output(&info).unwrap().active, 1);
    }

    #[test]
    fn sin_grilla_no_hay_estado() {
        assert_eq!(state_from_output(&json!({ "id": 1 })), None);
    }

    #[test]
    fn de_un_numero_a_la_celda_y_de_vuelta() {
        let state = state_from_output(&output(0, 0, 3, 2)).unwrap();
        assert_eq!(cell_of(0, &state), Some((0, 0)));
        assert_eq!(cell_of(4, &state), Some((1, 1)));
        assert_eq!(cell_of(5, &state), Some((2, 1)));
        assert_eq!(cell_of(6, &state), None, "no existe el séptimo");
        for index in 0..state.count {
            let (x, y) = cell_of(index, &state).unwrap();
            let back = state_from_output(&output(x, y, 3, 2)).unwrap();
            assert_eq!(back.active, index);
        }
    }

    const LIST: &str = "! model
  pc105           Generic 105-key PC

! layout
  us              English (US)
  latam           Spanish (Latin American)
  es              Spanish

! variant
  dvorak          us: English (Dvorak)
  deadtilde       latam: Spanish (Latin American, dead tilde)

! option
  grp             Switching to another layout
";

    #[test]
    fn la_lista_de_xkb_da_el_codigo_de_cada_nombre() {
        let codes = codes_by_name(LIST);
        assert_eq!(
            codes.get("Spanish (Latin American)").map(String::as_str),
            Some("latam")
        );
        assert_eq!(codes.get("English (US)").map(String::as_str), Some("us"));
        assert_eq!(
            codes.get("English (Dvorak)").map(String::as_str),
            Some("us")
        );
        assert_eq!(
            codes
                .get("Spanish (Latin American, dead tilde)")
                .map(String::as_str),
            Some("latam")
        );
        assert!(
            !codes.contains_key("Generic 105-key PC"),
            "un modelo no es una distribución"
        );
        assert!(!codes.contains_key("Switching to another layout"));
    }

    #[test]
    fn el_codigo_corto_son_dos_letras_en_mayusculas() {
        assert_eq!(short_label(Some("us"), "English (US)"), "US");
        assert_eq!(short_label(Some("latam"), "Spanish (Latin American)"), "LA");
        // Sin lista, las iniciales: nunca vacío.
        assert_eq!(short_label(None, "Spanish (Latin American)"), "SL");
        assert_eq!(short_label(Some(" "), "German"), "G");
    }

    #[test]
    fn la_distribucion_de_wayfire_se_lee_con_la_lista() {
        let codes = codes_by_name(LIST);
        let state = json!({
            "possible-layouts": ["Spanish (Latin American)", "English (US)"],
            "layout": "English (US)",
            "layout-index": 1
        });
        let layout = layout_from_state(&state, &codes).unwrap();
        assert_eq!(
            layout,
            KeyboardLayout {
                short: "US".into(),
                name: "English (US)".into(),
                index: 1,
                count: 2
            }
        );
        assert_eq!(next_index(&layout), 0, "da la vuelta");
    }

    #[test]
    fn con_una_sola_distribucion_la_siguiente_es_la_misma() {
        let codes = codes_by_name(LIST);
        let state = json!({
            "possible-layouts": ["Spanish (Latin American)"],
            "layout": "Spanish (Latin American)",
            "layout-index": 0
        });
        let layout = layout_from_state(&state, &codes).unwrap();
        assert_eq!(layout.short, "LA");
        assert_eq!(next_index(&layout), 0);
    }

    #[test]
    fn un_indice_fuera_de_rango_se_acota() {
        let state = json!({ "possible-layouts": ["A"], "layout": "A", "layout-index": 7 });
        assert_eq!(layout_from_state(&state, &HashMap::new()).unwrap().index, 0);
    }
}
