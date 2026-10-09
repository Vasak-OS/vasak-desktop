//! Las propiedades de una entrada de `com.canonical.dbusmenu`, leídas como dice
//! la especificación (`libdbusmenu-glib/dbus-menu.xml`).
//!
//! La especificación pide **no mandar lo que vale lo de omisión**, así que una
//! propiedad ausente no es «no sé»: es su valor por omisión. Los que importan:
//!
//! | propiedad | tipo | por omisión |
//! |---|---|---|
//! | `type` | `s` | `"standard"` (o `"separator"`) |
//! | `label` | `s` | `""`, con `_` de acelerador y `__` literal |
//! | `enabled`, `visible` | `b` | `true` |
//! | `icon-name` | `s` | `""` |
//! | `icon-data` | `ay` | vacío; los bytes de un PNG |
//! | `shortcut` | `aas` | vacío; `[["Control", "q"]]` |
//! | `toggle-type` | `s` | `""`; `"checkmark"` o `"radio"` |
//! | `toggle-state` | `i` | **−1**: 0 apagado, 1 encendido, otro indeterminado |
//! | `children-display` | `s` | `""`; `"submenu"` si tiene hijos |
//! | `disposition` | `s` | `"normal"`; `"informative"`, `"warning"`, `"alert"` |
//!
//! Lo mismo vale para `ItemsPropertiesUpdated`: trae las propiedades que
//! cambiaron y, aparte, las que se **quitaron**, que vuelven a su valor por
//! omisión. Por eso la entrada guarda lo que mandó el elemento sin interpretar
//! (`raw_*`) y lo que se dibuja se recalcula en [`finish`].

use crate::structs::{MenuDisposition, ToggleKind, ToggleState, TrayIcon, TrayMenu, TrayToggle};
use crate::tray::pixmap::menu_icon_png_base64;
use std::collections::HashMap;
use zbus::zvariant::Value;

fn unwrap_variant<'a>(v: &'a Value<'a>) -> &'a Value<'a> {
    match v {
        Value::Value(inner) => unwrap_variant(inner),
        other => other,
    }
}

pub fn get_string(v: &Value) -> Option<String> {
    match unwrap_variant(v) {
        Value::Str(s) => Some(s.as_str().to_string()),
        _ => None,
    }
}

pub fn get_bool(v: &Value) -> Option<bool> {
    match unwrap_variant(v) {
        Value::Bool(b) => Some(*b),
        _ => None,
    }
}

pub fn get_i32(v: &Value) -> Option<i32> {
    match unwrap_variant(v) {
        Value::I32(i) => Some(*i),
        Value::I16(i) => Some(*i as i32),
        Value::U8(i) => Some(*i as i32),
        Value::U16(i) => Some(*i as i32),
        Value::U32(i) => i32::try_from(*i).ok(),
        Value::I64(i) => i32::try_from(*i).ok(),
        _ => None,
    }
}

fn get_bytes(v: &Value) -> Option<Vec<u8>> {
    match unwrap_variant(v) {
        Value::Array(a) => a
            .iter()
            .map(|b| match b {
                Value::U8(b) => Some(*b),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

fn get_shortcut(v: &Value) -> Option<Vec<Vec<String>>> {
    let Value::Array(chords) = unwrap_variant(v) else {
        return None;
    };
    let chords: Vec<Vec<String>> = chords
        .iter()
        .filter_map(|chord| match unwrap_variant(chord) {
            Value::Array(keys) => {
                let keys: Vec<String> = keys.iter().filter_map(get_string).collect();
                (!keys.is_empty()).then_some(keys)
            }
            _ => None,
        })
        .collect();
    (!chords.is_empty()).then_some(chords)
}

/// La etiqueta de una entrada de dbusmenu, sin la marca del atajo de teclado.
///
/// La especificación marca la letra del atajo con un guion bajo adelante
/// («_Salir») y escribe `__` para un guion bajo de verdad. El nivel superior del
/// menú los borraba todos —y con ellos el literal— y el de los hijos ninguno,
/// así que en los submenús se leía «_Salir».
pub fn menu_label(raw: &str) -> String {
    let mut label = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '_' {
            if chars.peek() == Some(&'_') {
                chars.next();
                label.push('_');
            }
        } else {
            label.push(c);
        }
    }
    label
}

/// Casilla, radio o nada, y en qué estado.
///
/// `toggle-state` no significa nada sin `toggle-type`, y hay programas que lo
/// mandan en 0 o −1 en entradas comunes: tomarlo igual las dibujaba como
/// casillas destildadas. Con `toggle-type`, un estado ausente vale −1 —el de
/// omisión—, que es **indeterminado**: el elemento que quiere una casilla
/// apagada la manda en 0 (Discord lo hace así, medido).
pub fn menu_toggle(toggle_type: Option<&str>, toggle_state: Option<i32>) -> Option<TrayToggle> {
    let kind = match toggle_type {
        Some("checkmark") => ToggleKind::Checkmark,
        Some("radio") => ToggleKind::Radio,
        _ => return None,
    };
    let state = match toggle_state.unwrap_or(-1) {
        0 => ToggleState::Off,
        1 => ToggleState::On,
        _ => ToggleState::Indeterminate,
    };
    Some(TrayToggle { kind, state })
}

fn menu_disposition(value: &str) -> Option<MenuDisposition> {
    match value {
        "informative" => Some(MenuDisposition::Informative),
        "warning" => Some(MenuDisposition::Warning),
        "alert" => Some(MenuDisposition::Alert),
        _ => None,
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.trim().is_empty())
}

/// Una entrada nueva con los valores por omisión de la especificación.
pub fn new_entry(id: i32) -> TrayMenu {
    TrayMenu {
        id,
        enabled: true,
        visible: true,
        menu_type: "standard".to_string(),
        ..Default::default()
    }
}

/// Aplica una propiedad. Una de tipo equivocado cuenta como ausente: es lo que
/// haría el elemento si no la hubiera mandado.
pub fn apply_property(entry: &mut TrayMenu, key: &str, value: &Value) {
    match key {
        "label" => {
            entry.label = get_string(value)
                .map(|l| menu_label(&l))
                .unwrap_or_default()
        }
        "enabled" => entry.enabled = get_bool(value).unwrap_or(true),
        "visible" => entry.visible = get_bool(value).unwrap_or(true),
        "type" => entry.raw_type = get_string(value),
        "children-display" => entry.raw_children_display = get_string(value),
        "icon-name" => {
            let icon = entry.icon.get_or_insert_with(TrayIcon::default);
            icon.name = non_empty(get_string(value));
        }
        "icon-data" => {
            let icon = entry.icon.get_or_insert_with(TrayIcon::default);
            icon.data = get_bytes(value).and_then(|bytes| menu_icon_png_base64(&bytes));
        }
        "toggle-type" => entry.raw_toggle_type = get_string(value),
        "toggle-state" => entry.raw_toggle_state = get_i32(value),
        "shortcut" => entry.shortcut = get_shortcut(value),
        "disposition" => entry.disposition = get_string(value).and_then(|d| menu_disposition(&d)),
        _ => {}
    }
}

/// Lo que hace `ItemsPropertiesUpdated` con una propiedad quitada: vuelve a su
/// valor por omisión.
pub fn reset_property(entry: &mut TrayMenu, key: &str) {
    match key {
        "label" => entry.label.clear(),
        "enabled" => entry.enabled = true,
        "visible" => entry.visible = true,
        "type" => entry.raw_type = None,
        "children-display" => entry.raw_children_display = None,
        "icon-name" => {
            if let Some(icon) = entry.icon.as_mut() {
                icon.name = None;
            }
        }
        "icon-data" => {
            if let Some(icon) = entry.icon.as_mut() {
                icon.data = None;
            }
        }
        "toggle-type" => entry.raw_toggle_type = None,
        "toggle-state" => entry.raw_toggle_state = None,
        "shortcut" => entry.shortcut = None,
        "disposition" => entry.disposition = None,
        _ => {}
    }
}

/// Recalcula lo que depende de más de una propiedad.
pub fn finish(entry: &mut TrayMenu) {
    entry.menu_type = if entry.raw_type.as_deref() == Some("separator") {
        "separator".to_string()
    } else if entry.raw_children_display.as_deref() == Some("submenu") {
        "submenu".to_string()
    } else {
        "standard".to_string()
    };
    entry.toggle = menu_toggle(entry.raw_toggle_type.as_deref(), entry.raw_toggle_state);
    entry.icon = entry.icon.take().and_then(TrayIcon::non_empty);
}

/// Una entrada de `GetLayout` (`(ia{sv}av)`) con sus hijos.
pub fn parse_menu_value(v: &Value) -> Option<TrayMenu> {
    let Value::Structure(s) = unwrap_variant(v) else {
        return None;
    };
    let fields = s.fields();
    if fields.len() < 3 {
        return None;
    }

    let mut entry = new_entry(get_i32(&fields[0]).unwrap_or(0));
    if let Value::Dict(dict) = unwrap_variant(&fields[1]) {
        for (k, v) in dict.iter() {
            if let Some(key) = get_string(k) {
                apply_property(&mut entry, &key, v);
            }
        }
    }
    finish(&mut entry);

    let children: Vec<TrayMenu> = match unwrap_variant(&fields[2]) {
        Value::Array(a) => a.iter().filter_map(parse_menu_value).collect(),
        _ => Vec::new(),
    };
    entry.children = (!children.is_empty()).then_some(children);
    Some(entry)
}

/// La raíz de `GetLayout`, ya separada en sus tres partes.
pub fn parse_layout(
    id: i32,
    props: &HashMap<String, zbus::zvariant::OwnedValue>,
    children: &[zbus::zvariant::OwnedValue],
) -> TrayMenu {
    let mut entry = new_entry(id);
    for (key, value) in props {
        apply_property(&mut entry, key, value);
    }
    finish(&mut entry);
    let children: Vec<TrayMenu> = children
        .iter()
        .filter_map(|c| parse_menu_value(c))
        .collect();
    entry.children = (!children.is_empty()).then_some(children);
    entry
}

fn find_entry_mut(items: &mut [TrayMenu], id: i32) -> Option<&mut TrayMenu> {
    for item in items.iter_mut() {
        if item.id == id {
            return Some(item);
        }
        if let Some(found) = item
            .children
            .as_deref_mut()
            .and_then(|c| find_entry_mut(c, id))
        {
            return Some(found);
        }
    }
    None
}

/// Aplica `ItemsPropertiesUpdated` sobre el menú que ya se leyó.
///
/// Devuelve si tocó algo: una actualización de entradas que no están en lo
/// leído (un submenú que todavía no se pidió) no cambia lo que se dibuja.
pub fn apply_items_properties_updated(
    items: &mut [TrayMenu],
    updated: &[(i32, HashMap<String, zbus::zvariant::OwnedValue>)],
    removed: &[(i32, Vec<String>)],
) -> bool {
    let mut touched = false;
    for (id, props) in updated {
        if let Some(entry) = find_entry_mut(items, *id) {
            for (key, value) in props {
                apply_property(entry, key, value);
            }
            finish(entry);
            touched = true;
        }
    }
    for (id, keys) in removed {
        if let Some(entry) = find_entry_mut(items, *id) {
            for key in keys {
                reset_property(entry, key);
            }
            finish(entry);
            touched = true;
        }
    }
    touched
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tray::pixmap::tests::png_bytes;
    use zbus::zvariant::Array;

    fn entry_with(props: Vec<(&str, Value<'static>)>) -> TrayMenu {
        let map: HashMap<String, Value> =
            props.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        let value = Value::from((7i32, map, Vec::<Value>::new()));
        parse_menu_value(&value).expect("la entrada se lee")
    }

    #[test]
    fn la_marca_del_atajo_se_va_y_el_guion_bajo_literal_queda() {
        assert_eq!(menu_label("_Salir"), "Salir");
        assert_eq!(menu_label("Abrir _carpeta"), "Abrir carpeta");
        assert_eq!(menu_label("mi__archivo"), "mi_archivo");
        assert_eq!(menu_label("Sin atajo"), "Sin atajo");
    }

    #[test]
    fn sin_propiedades_es_una_entrada_comun_habilitada_y_visible() {
        let e = entry_with(vec![]);
        assert_eq!(e.menu_type, "standard");
        assert!(e.enabled && e.visible);
        assert_eq!(
            (e.toggle, e.icon, e.shortcut, e.disposition),
            (None, None, None, None)
        );
    }

    #[test]
    fn solo_se_tilda_lo_que_es_tildable() {
        use ToggleKind::*;
        use ToggleState::*;
        assert_eq!(
            menu_toggle(Some("checkmark"), Some(1)),
            Some(TrayToggle {
                kind: Checkmark,
                state: On
            })
        );
        assert_eq!(
            menu_toggle(Some("radio"), Some(0)),
            Some(TrayToggle {
                kind: Radio,
                state: Off
            })
        );
        assert_eq!(menu_toggle(None, Some(0)), None);
        assert_eq!(menu_toggle(Some(""), Some(-1)), None);
    }

    #[test]
    fn toggle_state_menos_uno_o_ausente_es_indeterminado() {
        let state = |s| menu_toggle(Some("checkmark"), s).unwrap().state;
        assert_eq!(state(Some(-1)), ToggleState::Indeterminate);
        assert_eq!(state(None), ToggleState::Indeterminate);
        assert_eq!(state(Some(2)), ToggleState::Indeterminate);
        let radio = entry_with(vec![
            ("toggle-type", Value::from("radio")),
            ("toggle-state", Value::from(-1i32)),
        ]);
        assert_eq!(radio.toggle.unwrap().state, ToggleState::Indeterminate);
    }

    #[test]
    fn el_parser_completo_conserva_el_guion_bajo_literal() {
        assert_eq!(
            entry_with(vec![("label", Value::from("mi__archivo"))]).label,
            "mi_archivo"
        );
    }

    #[test]
    fn icono_por_nombre_por_png_o_los_dos() {
        let named = entry_with(vec![("icon-name", Value::from("document-open"))]);
        assert_eq!(named.icon.unwrap().name.as_deref(), Some("document-open"));

        let png = Value::Array(Array::from(png_bytes(16, 16)));
        let both = entry_with(vec![("icon-name", Value::from("x")), ("icon-data", png)]);
        let icon = both.icon.unwrap();
        assert!(icon.name.is_some() && icon.data.is_some());

        // Un nombre vacío no es un icono.
        assert_eq!(entry_with(vec![("icon-name", Value::from(""))]).icon, None);
    }

    #[test]
    fn un_png_invalido_en_el_menu_no_deja_icono() {
        let junk = Value::Array(Array::from(b"no soy un png".to_vec()));
        assert_eq!(entry_with(vec![("icon-data", junk)]).icon, None);
        // Con nombre al lado, queda el nombre.
        let junk = Value::Array(Array::from(b"roto".to_vec()));
        let e = entry_with(vec![
            ("icon-data", junk),
            ("icon-name", Value::from("edit-copy")),
        ]);
        assert_eq!(
            e.icon,
            Some(TrayIcon {
                name: Some("edit-copy".into()),
                data: None
            })
        );
    }

    #[test]
    fn el_atajo_se_lee_como_pulsaciones() {
        let shortcut = Value::from(vec![vec!["Control", "q"], vec!["Alt", "X"]]);
        let e = entry_with(vec![("shortcut", shortcut)]);
        assert_eq!(
            e.shortcut,
            Some(vec![
                vec!["Control".to_string(), "q".to_string()],
                vec!["Alt".to_string(), "X".to_string()]
            ])
        );
        // Vacío es como no tenerlo.
        assert_eq!(
            entry_with(vec![("shortcut", Value::from(Vec::<Vec<&str>>::new()))]).shortcut,
            None
        );
    }

    #[test]
    fn la_disposicion_normal_no_se_manda() {
        let d = |v: &str| entry_with(vec![("disposition", Value::from(v.to_string()))]).disposition;
        assert_eq!(d("normal"), None);
        assert_eq!(d("informative"), Some(MenuDisposition::Informative));
        assert_eq!(d("warning"), Some(MenuDisposition::Warning));
        assert_eq!(d("alert"), Some(MenuDisposition::Alert));
        assert_eq!(d("x-vendor-algo"), None);
    }

    #[test]
    fn submenu_y_separador() {
        assert_eq!(
            entry_with(vec![("children-display", Value::from("submenu"))]).menu_type,
            "submenu"
        );
        assert_eq!(
            entry_with(vec![("type", Value::from("separator"))]).menu_type,
            "separator"
        );
    }

    #[test]
    fn un_tipo_equivocado_cuenta_como_ausente() {
        let e = entry_with(vec![
            ("enabled", Value::from("no")),
            ("label", Value::from(3i32)),
        ]);
        assert!(e.enabled);
        assert_eq!(e.label, "");
    }

    fn owned(v: Value<'static>) -> zbus::zvariant::OwnedValue {
        zbus::zvariant::OwnedValue::try_from(v).unwrap()
    }

    fn tree() -> Vec<TrayMenu> {
        let mut parent = new_entry(1);
        parent.raw_children_display = Some("submenu".into());
        let mut child = new_entry(2);
        child.label = "Silenciar".into();
        child.raw_toggle_type = Some("checkmark".into());
        child.raw_toggle_state = Some(0);
        finish(&mut child);
        finish(&mut parent);
        parent.children = Some(vec![child]);
        vec![parent]
    }

    #[test]
    fn items_properties_updated_cambia_la_entrada_aunque_este_en_un_submenu() {
        let mut items = tree();
        let mut props = HashMap::new();
        props.insert("toggle-state".to_string(), owned(Value::from(1i32)));
        props.insert("label".to_string(), owned(Value::from("_Silenciado")));
        assert!(apply_items_properties_updated(
            &mut items,
            &[(2, props)],
            &[]
        ));
        let child = &items[0].children.as_ref().unwrap()[0];
        assert_eq!(child.toggle.unwrap().state, ToggleState::On);
        assert_eq!(child.label, "Silenciado");
    }

    #[test]
    fn una_propiedad_quitada_vuelve_a_su_valor_por_omision() {
        let mut items = tree();
        // Quitar `toggle-state` es volver a −1: indeterminado.
        assert!(apply_items_properties_updated(
            &mut items,
            &[],
            &[(2, vec!["toggle-state".into()])]
        ));
        let child = &items[0].children.as_ref().unwrap()[0];
        assert_eq!(child.toggle.unwrap().state, ToggleState::Indeterminate);
        // Quitar `toggle-type` la deja de ser casilla.
        apply_items_properties_updated(&mut items, &[], &[(2, vec!["toggle-type".into()])]);
        assert_eq!(items[0].children.as_ref().unwrap()[0].toggle, None);
    }

    #[test]
    fn una_actualizacion_de_algo_que_no_se_leyo_no_toca_nada() {
        let mut items = tree();
        let before = items.clone();
        let mut props = HashMap::new();
        props.insert("label".to_string(), owned(Value::from("x")));
        assert!(!apply_items_properties_updated(
            &mut items,
            &[(99, props)],
            &[(98, vec!["label".into()])]
        ));
        assert_eq!(items, before);
    }
}
