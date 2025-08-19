use super::{WindowInfo, WindowManagerBackend};
use std::collections::HashMap;
// Ordering ya no es necesario si quitamos AtomicBool
// use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::CURRENT_TIME;

pub struct X11Manager {
    conn: Arc<x11rb::rust_connection::RustConnection>,
    root: Window,
    atoms: HashMap<&'static str, Atom>, // Átomos cacheados
}

impl X11Manager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (conn, screen_num) = x11rb::connect(None)?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;
        let conn_arc = Arc::new(conn);

        let atom_names = [
            "_NET_CLIENT_LIST",
            "_NET_WM_NAME",
            "WM_NAME",
            "UTF8_STRING",
            "_NET_WM_STATE",
            "_NET_WM_STATE_HIDDEN",
            "_NET_WM_STATE_SKIP_TASKBAR",
            // "_NET_WM_STATE_SKIP_PAGER",
            "_NET_WM_STATE_MODAL",
            "_NET_WM_STATE_DEMANDS_ATTENTION",
            "_NET_ACTIVE_WINDOW",
            "_NET_WM_WINDOW_TYPE",
            "_NET_WM_WINDOW_TYPE_DOCK",
            "_NET_WM_WINDOW_TYPE_DESKTOP",
            "_NET_WM_WINDOW_TYPE_TOOLBAR",
            "_NET_WM_WINDOW_TYPE_MENU",
            "_NET_WM_WINDOW_TYPE_SPLASH",
            "_NET_WM_WINDOW_TYPE_DIALOG",
            "_NET_WM_WINDOW_TYPE_UTILITY",
            "_NET_WM_WINDOW_TYPE_TOOLTIP",
            "_NET_WM_WINDOW_TYPE_NOTIFICATION",
            "_NET_WM_WINDOW_TYPE_DROPDOWN_MENU",
            "_NET_WM_WINDOW_TYPE_POPUP_MENU",
            "WM_CLASS",
        ];

        let mut atoms = HashMap::new();
        for name in atom_names.iter() {
            let interned_atom = conn_arc.intern_atom(false, name.as_bytes())?.reply()?;
            atoms.insert(*name, interned_atom.atom);
        }

        Ok(X11Manager {
            conn: conn_arc,
            root,
            atoms,
        })
    }

    fn get_window_title(&self, win: Window) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(net_wm_name_atom) = self.atoms.get("_NET_WM_NAME") {
            if let Some(utf8_string_atom) = self.atoms.get("UTF8_STRING") {
                let reply = self
                    .conn
                    .get_property(
                        false,
                        win,
                        *net_wm_name_atom,
                        *utf8_string_atom,
                        0,
                        u32::MAX,
                    )?
                    .reply()?;
                if !reply.value.is_empty() {
                    return Ok(String::from_utf8_lossy(&reply.value).into_owned());
                }
            }
        }

        if let Some(wm_name_atom) = self.atoms.get("WM_NAME") {
            let reply = self
                .conn
                .get_property(false, win, *wm_name_atom, AtomEnum::STRING, 0, u32::MAX)?
                .reply()?;
            if !reply.value.is_empty() {
                return Ok(String::from_utf8_lossy(&reply.value).into_owned()); // from_utf8_lossy es seguro
            }
        }

        Ok(String::new())
    }

    fn get_window_state(&self, win: Window) -> Result<Vec<Atom>, Box<dyn std::error::Error>> {
        let net_wm_state_atom = self
            .atoms
            .get("_NET_WM_STATE")
            .ok_or("_NET_WM_STATE atom not found in cache")?;
        let reply = self
            .conn
            .get_property(false, win, *net_wm_state_atom, AtomEnum::ATOM, 0, u32::MAX)?
            .reply()?;

        Ok(reply
            .value32()
            .map(|iter| iter.collect())
            .unwrap_or_default())
    }

    fn get_window_class(&self, win: Window) -> Result<String, Box<dyn std::error::Error>> {
        let cookie = self.conn.get_property(
            false,
            win,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            0,
            u32::MAX,
        )?;
        let reply = cookie.reply()?;

        if let Some(data) = reply.value8() {
            let data_vec: Vec<u8> = data.collect();
            let mut parts = data_vec.split(|&b| b == 0);
            if let Some(first_part_bytes) = parts.next() {
                if !first_part_bytes.is_empty() {
                    return Ok(String::from_utf8_lossy(first_part_bytes).into_owned());
                }
                if let Some(second_part_bytes) = parts.next() {
                    if !second_part_bytes.is_empty() {
                        return Ok(String::from_utf8_lossy(second_part_bytes).into_owned());
                    }
                }
            }
        }

        Ok(String::new())
    }

    fn is_window_focused(&self, window: Window) -> Result<bool, Box<dyn std::error::Error>> {
        let net_active_window_atom = self
            .atoms
            .get("_NET_ACTIVE_WINDOW")
            .ok_or("_NET_ACTIVE_WINDOW atom not found in cache")?;
        let active_window_reply = self
            .conn
            .get_property(
                false,
                self.root,
                *net_active_window_atom,
                AtomEnum::WINDOW,
                0,
                1,
            )?
            .reply()?;

        Ok(active_window_reply.value32().and_then(|mut v| v.next()) == Some(window))
    }

    fn change_net_wm_state(
        &self,
        win: Window,
        action: u32,
        property_atom: Atom,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let net_wm_state_atom = self
            .atoms
            .get("_NET_WM_STATE")
            .ok_or("_NET_WM_STATE atom not found in cache")?;

        let event = ClientMessageEvent {
            response_type: CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window: win,
            type_: *net_wm_state_atom,
            data: ClientMessageData::from([action, property_atom, 0, 2, 0]),
        };

        self.conn.send_event(
            false,
            self.root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )?;
        Ok(())
    }

    fn activate_window_ewmh(&self, window: Window) -> Result<(), Box<dyn std::error::Error>> {
        let net_active_window_atom = self
            .atoms
            .get("_NET_ACTIVE_WINDOW")
            .ok_or("_NET_ACTIVE_WINDOW atom not found in cache")?;

        let current_active_window_for_source: u32 = 0;

        let event = ClientMessageEvent {
            response_type: CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window,
            type_: *net_active_window_atom,
            data: ClientMessageData::from([
                2,
                CURRENT_TIME,
                current_active_window_for_source,
                0,
                0,
            ]),
        };

        self.conn.send_event(
            false,
            self.root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )?;
        Ok(())
    }

    fn should_show_window(&self, win: Window) -> Result<bool, Box<dyn std::error::Error>> {
        let attributes = self.conn.get_window_attributes(win)?.reply()?;
        if attributes.override_redirect {
            log::debug!("Window {} filtered: override_redirect", win);
            return Ok(false);
        }

        // Check window type - be more permissive
        if let Some(net_wm_window_type_atom) = self.atoms.get("_NET_WM_WINDOW_TYPE") {
            if let Ok(reply) = self
                .conn
                .get_property(
                    false,
                    win,
                    *net_wm_window_type_atom,
                    AtomEnum::ATOM,
                    0,
                    u32::MAX,
                )?
                .reply()
            {
                if let Some(types) = reply.value32() {
                    for window_type_atom_value in types {
                        // Only filter out these specific types - removed TOOLBAR and UTILITY
                        if Some(&window_type_atom_value)
                            == self.atoms.get("_NET_WM_WINDOW_TYPE_DOCK")
                            || Some(&window_type_atom_value)
                                == self.atoms.get("_NET_WM_WINDOW_TYPE_TOOLBAR")
                            || Some(&window_type_atom_value)
                                == self.atoms.get("_NET_WM_WINDOW_TYPE_DESKTOP")
                            || Some(&window_type_atom_value)
                                == self.atoms.get("_NET_WM_WINDOW_TYPE_SPLASH")
                            || Some(&window_type_atom_value)
                                == self.atoms.get("_NET_WM_WINDOW_TYPE_UTILITY")
                            || Some(&window_type_atom_value)
                                == self.atoms.get("_NET_WM_WINDOW_TYPE_DROPDOWN_MENU")
                            || Some(&window_type_atom_value)
                                == self.atoms.get("_NET_WM_WINDOW_TYPE_POPUP_MENU")
                            || Some(&window_type_atom_value)
                                == self.atoms.get("_NET_WM_WINDOW_TYPE_TOOLTIP")
                            || Some(&window_type_atom_value)
                                == self.atoms.get("_NET_WM_WINDOW_TYPE_NOTIFICATION")
                            || Some(&window_type_atom_value)
                                == self.atoms.get("_NET_WM_WINDOW_TYPE_MENU")
                        {
                            log::debug!("Window {} filtered: unwanted window type", win);
                            return Ok(false);
                        }
                    }
                }
            }
        }

        // Be more permissive with skip taskbar - many legitimate apps set this
        let window_state = self.get_window_state(win)?;
        if let Some(skip_taskbar_atom) = self.atoms.get("_NET_WM_STATE_SKIP_TASKBAR") {
            if window_state.contains(skip_taskbar_atom) {
                log::debug!("Window {} has SKIP_TASKBAR but allowing it", win);
                return Ok(false);
            }
        }

        let class_name = self.get_window_class(win)?;
        let skip_classes = [
            "vpanel",
            "tauri",
            "trayer",
            "plank",
            "docky",
            "cairo-dock",
            "tint2",
            "polybar",
            "lemonbar",
            "vmenu",
            "vasak-control-center",
        ];
        if skip_classes
            .iter()
            .any(|c| class_name.to_lowercase().contains(c))
        {
            log::debug!("Window {} filtered: class {} in skip list", win, class_name);
            return Ok(false);
        }

        Ok(true)
    }
}

impl WindowManagerBackend for X11Manager {
    fn get_window_list(&mut self) -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>> {
        // get_required_atoms ya no se llama aquí
        let net_client_list_atom = self
            .atoms
            .get("_NET_CLIENT_LIST")
            .ok_or("_NET_CLIENT_LIST atom not found in cache")?;
        let net_wm_state_hidden_atom = self
            .atoms
            .get("_NET_WM_STATE_HIDDEN")
            .ok_or("_NET_WM_STATE_HIDDEN atom not found in cache")?;

        let reply = self
            .conn
            .get_property(
                false,
                self.root,
                *net_client_list_atom,
                AtomEnum::WINDOW,
                0,
                u32::MAX,
            )?
            .reply()?;

        let windows_prop: Vec<Window> =
            reply.value32().map_or_else(Vec::new, |iter| iter.collect());
        let mut window_list = Vec::new();

        for win in windows_prop {
            let class_name = self.get_window_class(win).unwrap_or_default();
            let title = self.get_window_title(win).unwrap_or_default();

            log::debug!(
                "Checking window: {} (class: {}, title: {})",
                win,
                class_name,
                title
            );

            if !self.should_show_window(win)? {
                log::debug!(
                    "Window {} filtered out (class: {}, title: {})",
                    win,
                    class_name,
                    title
                );
                continue;
            }

            log::debug!(
                "Window {} included (class: {}, title: {})",
                win,
                class_name,
                title
            );

            let state = self.get_window_state(win)?; // Llama a la versión que usa self.atoms
            let class_name = self.get_window_class(win).unwrap_or_default();

            let demands_attention =
                if let Some(da_atom) = self.atoms.get("_NET_WM_STATE_DEMANDS_ATTENTION") {
                    Some(state.contains(da_atom))
                } else {
                    None
                };

            window_list.push(WindowInfo {
                id: win.to_string(),
                title,
                is_minimized: state.contains(net_wm_state_hidden_atom),
                icon: class_name,
                demands_attention,
            });
        }

        Ok(window_list)
    }

    fn toggle_window(&self, win_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let window_to_toggle = win_id.parse::<Window>()?;

        let net_wm_state_hidden_atom =
            self.atoms
                .get("_NET_WM_STATE_HIDDEN")
                .copied()
                .ok_or_else(|| {
                    Into::<Box<dyn std::error::Error>>::into(
                        "_NET_WM_STATE_HIDDEN atom not found in cache",
                    )
                })?;

        let net_wm_state_demands_attention_atom =
            self.atoms.get("_NET_WM_STATE_DEMANDS_ATTENTION").copied();

        let window_state = self.get_window_state(window_to_toggle)?;
        let is_hidden = window_state.contains(&net_wm_state_hidden_atom);
        let is_focused = self.is_window_focused(window_to_toggle)?;

        log::debug!(
            "Toggle window {}: hidden={}, focused={}",
            window_to_toggle,
            is_hidden,
            is_focused
        );

        if is_hidden {
            // Window is minimized - restore and activate it
            log::debug!("Unminimizing window {}", window_to_toggle);
            self.change_net_wm_state(window_to_toggle, 0, net_wm_state_hidden_atom)?;

            if let Some(da_atom) = net_wm_state_demands_attention_atom {
                self.change_net_wm_state(window_to_toggle, 1, da_atom)?;
            }

            self.activate_window_ewmh(window_to_toggle)?;

            if let Some(da_atom) = net_wm_state_demands_attention_atom {
                self.change_net_wm_state(window_to_toggle, 0, da_atom)?;
            }
        } else if is_focused {
            // Window is focused - minimize it
            log::debug!("Minimizing focused window {}", window_to_toggle);
            self.change_net_wm_state(window_to_toggle, 1, net_wm_state_hidden_atom)?;
        } else {
            // Window is not focused - bring it to front
            log::debug!("Activating unfocused window {}", window_to_toggle);
            if let Some(da_atom) = net_wm_state_demands_attention_atom {
                if window_state.contains(&da_atom) {
                    self.change_net_wm_state(window_to_toggle, 0, da_atom)?;
                }
            }
            self.activate_window_ewmh(window_to_toggle)?;
        }

        self.conn.flush()?;
        Ok(())
    }

    fn setup_event_monitoring(&mut self, tx: Sender<()>) -> Result<(), Box<dyn std::error::Error>> {
        let conn_clone = self.conn.clone();
        let root_window = self.root;

        let event_mask = EventMask::SUBSTRUCTURE_NOTIFY | EventMask::PROPERTY_CHANGE;
        conn_clone.change_window_attributes(
            root_window,
            &ChangeWindowAttributesAux::new().event_mask(event_mask),
        )?;
        conn_clone.flush()?;

        thread::spawn(move || {
            loop {
                match conn_clone.wait_for_event() {
                    Ok(event) => {
                        match event {
                            Event::PropertyNotify(_ev) => {
                                if tx.send(()).is_err() { break; }
                            }
                            Event::CreateNotify(_) // CreateNotify, DestroyNotify, etc. en hijos de root
                            | Event::DestroyNotify(_) => {
                                if tx.send(()).is_err() { break; }
                            }
                            _ => {}
                        }
                    }
                    Err(_) => {
                        eprintln!("[EventMonitor] Error esperando evento o conexión cerrada.");
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}
