use super::Applet;
use crate::dbus_pool::DbusPool;
use crate::structs::BatteryInfo;
use async_trait::async_trait;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use zbus::zvariant::OwnedValue;
use zbus::{Connection, Message, MessageStream};

pub struct BatteryApplet;

// Cached battery state for timeout fallback
static BATTERY_CACHE: Mutex<Option<BatteryInfo>> = Mutex::new(None);
// Cached device path (discovered at startup)
static BATTERY_DEVICE_PATH: Mutex<Option<String>> = Mutex::new(None);

#[async_trait]
impl Applet for BatteryApplet {
    fn name(&self) -> &'static str {
        "battery"
    }

    async fn start(&self, app_handle: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        // Check SysFS availability first (cheap, no D-Bus)
        let sysfs_battery = "/sys/class/power_supply/BAT0";
        let has_sysfs_battery = Path::new(sysfs_battery).exists();
        let has_sysfs = has_sysfs_battery
            || Path::new("/sys/class/power_supply/AC0").exists();

        // Try D-Bus path
        if let Some(conn) = get_system_connection(&app_handle).await {
            if let Some(path) = find_battery_path(&conn).await {
                // Cache the path and use D-Bus monitoring
                BATTERY_DEVICE_PATH.lock().unwrap().replace(path.clone());

                // Emit initial state
                if let Some(info) = get_battery_info_with_conn(&conn).await {
                    let _ = app_handle.emit("battery-update", &info);
                }

                self.run_dbus_loop(app_handle, conn, path).await;
                return Ok(());
            }
        }

        // D-Bus unavailable or no UPower battery — fall back to SysFS
        if has_sysfs {
            // Emit initial state from SysFS
            if let Some(info) = read_sysfs_battery_info() {
                let _ = app_handle.emit("battery-update", &info);
            }

            self.run_sysfs_loop(app_handle).await;
        } else {
            // No battery found at all — monitor occasionally in case one appears
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;

                if let Some(info) = read_sysfs_battery_info() {
                    let _ = app_handle.emit("battery-update", &info);
                }
            }
        }

        Ok(())
    }
}

impl BatteryApplet {
    async fn run_sysfs_loop(&self, app_handle: AppHandle) {
        let mut last_info: Option<BatteryInfo> = None;
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;

            if let Some(current_info) = read_sysfs_battery_info() {
                let should_emit = match &last_info {
                    None => true,
                    Some(last) => {
                        (last.percentage - current_info.percentage).abs() > 0.1
                            || last.is_charging != current_info.is_charging
                            || last.state != current_info.state
                    }
                };

                if should_emit {
                    let _ = app_handle.emit("battery-update", &current_info);
                    last_info = Some(current_info);
                }
            }
        }
    }

    async fn run_dbus_loop(
        &self,
        app_handle: AppHandle,
        mut conn: Connection,
        mut path: String,
    ) {
        let mut reconnect_attempts = 0u32;
        let max_reconnects = 5;

        loop {
            match self
                .monitor_dbus_with_reconnect(
                    &app_handle,
                    conn.clone(),
                    path.clone(),
                    reconnect_attempts,
                )
                .await
            {
                Ok(_) => {
                    log::info!("[battery] D-Bus monitor ended normally");
                    break;
                }
                Err(e) => {
                    reconnect_attempts += 1;
                    if reconnect_attempts >= max_reconnects {
                        log::error!("[battery] Max reconnection attempts reached: {}", e);
                        let _ = app_handle.emit(
                            "dbus-status",
                            serde_json::json!({
                                "service": "battery",
                                "status": "failed",
                                "message": "No se pudo conectar a UPower"
                            }),
                        );
                        // Fallback to polling
                        self.run_sysfs_loop(app_handle).await;
                        break;
                    }
                    log::warn!(
                        "[battery] Connection lost (attempt {}): {}. Reconnecting...",
                        reconnect_attempts,
                        e
                    );
                    let _ = app_handle.emit(
                        "dbus-status",
                        serde_json::json!({
                            "service": "battery",
                            "status": "reconnecting",
                            "attempt": reconnect_attempts
                        }),
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(
                        2u64.pow(reconnect_attempts.min(3)),
                    ))
                    .await;

                    // Try to get a fresh connection from the pool
                    if let Some(new_conn) = get_system_connection(&app_handle).await {
                        if let Some(p) = find_battery_path(&new_conn).await {
                            BATTERY_DEVICE_PATH.lock().unwrap().replace(p.clone());
                            path = p;
                        }
                        conn = new_conn;
                    }
                }
            }
        }
    }

    async fn monitor_dbus_with_reconnect(
        &self,
        app_handle: &AppHandle,
        conn: Connection,
        path: String,
        attempt: u32,
    ) -> Result<(), String> {
        if attempt > 0 {
            log::info!(
                "[battery] Reconnected successfully after {} attempts",
                attempt
            );
            let _ = app_handle.emit(
                "dbus-status",
                serde_json::json!({
                    "service": "battery",
                    "status": "connected"
                }),
            );
        }

        let mut last_info: Option<BatteryInfo> = None;

        // Asking for the signal is what was missing, and without it the whole
        // loop below was decoration: a plain `MessageStream::from(&conn)` only
        // yields what the bus decided to send us, and broadcast signals are only
        // sent to connections that registered a match rule for them. UPower's
        // PropertiesChanged never arrived, so the battery froze at whatever it
        // read when the desktop started and the five-second branch below just
        // re-emitted that same cached value for the rest of the session.
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface("org.freedesktop.DBus.Properties")
            .map_err(|e| format!("bad match rule: {e}"))?
            .member("PropertiesChanged")
            .map_err(|e| format!("bad match rule: {e}"))?
            .path(path.as_str())
            .map_err(|e| format!("bad match rule: {e}"))?
            .build();

        let mut stream = MessageStream::for_match_rule(rule, &conn, Some(16))
            .await
            .map_err(|e| format!("could not subscribe to UPower: {e}"))?;

        loop {
            tokio::select! {
                biased;

                msg_result = stream.next() => {
                    let msg = match msg_result {
                        Some(Ok(msg)) => msg,
                        Some(Err(e)) => {
                            log::error!("[battery] Stream error: {}", e);
                            return Err(format!("D-Bus stream error: {}", e));
                        }
                        None => {
                            return Err("D-Bus connection closed".to_string());
                        }
                    };

                    let header = msg.header();
                    if let (Some(interface), Some(member), Some(obj_path)) = (header.interface(), header.member(), header.path()) {
                        if interface.as_str() == "org.freedesktop.DBus.Properties" &&
                           member.as_str() == "PropertiesChanged" &&
                           obj_path.as_str() == path {
                               let info = match Self::handle_properties_changed(&msg, &conn).await {
                                   Some(i) => Some(i),
                                   None => {
                                       // Signal parsing failed or invalidated properties present,
                                       // fall back to full GetAll
                                       get_battery_info_with_conn(&conn).await
                                   }
                               };
                               Self::emit_if_changed(app_handle, &mut last_info, &info).await;
                        }
                    }
                }

                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    // A safety net, not the mechanism: with the match rule in
                    // place the signals do the work. Re-reading the cache would
                    // only ever repeat what is already on screen, so this asks
                    // the device again — the cheap sysfs read, since it costs
                    // nothing and catches a signal we somehow missed.
                    let info = read_sysfs_battery_info()
                        .or_else(|| BATTERY_CACHE.lock().unwrap().clone());

                    let info = if info.is_some() {
                        info
                    } else {
                        // Initial timeout without cache: emit has_battery: false, retry in 5s
                        Some(BatteryInfo {
                            has_battery: false,
                            percentage: 0.0,
                            state: "Unknown".to_string(),
                            time_to_empty: None,
                            time_to_full: None,
                            is_present: false,
                            is_charging: false,
                            vendor: None,
                            model: None,
                            technology: None,
                            energy: None,
                            energy_full: None,
                            energy_full_design: None,
                            voltage: None,
                            temperature: None,
                            serial: None,
                        })
                    };
                    Self::emit_if_changed(app_handle, &mut last_info, &info).await;
                }
            }
        }
    }

    /// Handle a PropertiesChanged signal by merging changed properties into the cache.
    /// Returns Some(BatteryInfo) if the merge was successful.
    /// Returns None if:
    ///   - The signal body couldn't be parsed
    ///   - There are invalidated properties (requiring a full GetAll refresh)
    ///   - No existing cache to merge into
    async fn handle_properties_changed(
        msg: &Message,
        conn: &Connection,
    ) -> Option<BatteryInfo> {
        // Parse the PropertiesChanged signal body:
        // (interface_name: String, changed_properties: HashMap<String, OwnedValue>, invalidated_properties: Vec<String>)
        let body: (String, HashMap<String, OwnedValue>, Vec<String>) =
            match msg.body().deserialize() {
                Ok(b) => b,
                Err(e) => {
                    log::debug!("[battery] Failed to parse PropertiesChanged body: {}", e);
                    return None;
                }
            };

        let (_interface_name, changed_properties, invalidated_properties) = body;

        // If any properties are invalidated (removed without new value), we need a full refresh
        if !invalidated_properties.is_empty() {
            log::debug!(
                "[battery] Invalidated properties detected: {:?}, doing full refresh",
                invalidated_properties
            );
            return None;
        }

        // If no changed properties, nothing to do
        if changed_properties.is_empty() {
            // Return current cache as-is
            return BATTERY_CACHE.lock().unwrap().clone();
        }

        // Get the current cached state to merge into
        let mut cached = match BATTERY_CACHE.lock().unwrap().clone() {
            Some(info) => info,
            None => {
                // No cache exists yet, can't merge - need a full GetAll
                log::debug!("[battery] No cache for signal merge, falling back to GetAll");
                return None;
            }
        };

        // Merge each changed property into the cached BatteryInfo
        merge_properties_into_cache(&mut cached, &changed_properties);

        // Update the cache with merged state
        *BATTERY_CACHE.lock().unwrap() = Some(cached.clone());

        // We don't need the connection for the merge itself, but it's available
        // in case future logic needs it
        let _ = conn;

        Some(cached)
    }

    async fn emit_if_changed(
        app_handle: &AppHandle,
        last_info: &mut Option<BatteryInfo>,
        current: &Option<BatteryInfo>,
    ) {
        let should_emit = match (last_info.as_ref(), current) {
            (_, None) => false,
            (None, Some(_)) => true,
            (Some(last), Some(cur)) => {
                (last.percentage - cur.percentage).abs() > 0.5
                    || last.is_charging != cur.is_charging
                    || last.state != cur.state
            }
        };

        if should_emit {
            let _ = app_handle.emit("battery-update", current);
            *last_info = current.clone();
        }
    }
}

/// Read battery info from SysFS without any D-Bus dependency.
/// Used as fallback when UPower or system bus is unavailable.
fn read_sysfs_battery_info() -> Option<BatteryInfo> {
    let bat_path = Path::new("/sys/class/power_supply/BAT0");
    if !bat_path.exists() {
        return None;
    }

    let read_str = |file: &str| -> Option<String> {
        fs::read_to_string(bat_path.join(file)).ok().map(|s| s.trim().to_string())
    };

    let capacity: f64 = read_str("capacity")?.parse().ok()?;
    let status = read_str("status")?.to_lowercase();
    let present_str = read_str("present").unwrap_or_default();

    let is_present = present_str == "1";
    let is_charging = status == "charging";
    let state = match status.as_str() {
        "charging" => "Charging",
        "discharging" => "Discharging",
        "full" => "FullyCharged",
        _ => "Unknown",
    }.to_string();

    // Time estimates (seconds in sysfs, or 0 if unavailable)
    let time_to_empty = read_str("time_to_empty_now")
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&v| v > 0);
    let time_to_full = read_str("time_to_full_now")
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&v| v > 0);

    // Optional metadata
    let vendor = read_str("manufacturer");
    let model = read_str("model_name");
    let technology = read_str("technology");
    let serial = read_str("serial_number");

    // Energy (sysfs: µWh → Wh)
    let energy = read_str("energy_now")
        .and_then(|s| s.parse::<f64>().ok())
        .map(|v| v / 1_000_000.0);
    let energy_full = read_str("energy_full")
        .and_then(|s| s.parse::<f64>().ok())
        .map(|v| v / 1_000_000.0);
    let energy_full_design = read_str("energy_full_design")
        .and_then(|s| s.parse::<f64>().ok())
        .map(|v| v / 1_000_000.0);

    // Voltage (sysfs: µV → V)
    let voltage = read_str("voltage_now")
        .and_then(|s| s.parse::<f64>().ok())
        .map(|v| v / 1_000_000.0);

    // Temperature (sysfs: tenths of °C → °C)
    let temperature = read_str("temp")
        .and_then(|s| s.parse::<f64>().ok())
        .map(|v| v / 10.0);

    Some(BatteryInfo {
        has_battery: true,
        percentage: capacity,
        state,
        time_to_empty,
        time_to_full,
        is_present,
        is_charging,
        vendor,
        model,
        technology,
        energy,
        energy_full,
        energy_full_design,
        voltage,
        temperature,
        serial,
    })
}

/// Get system bus connection from DbusPool managed state.
/// Falls back to creating a new connection if pool is unavailable.
async fn get_system_connection(app_handle: &AppHandle) -> Option<Connection> {
    // Try to get from DbusPool managed state
    if let Some(pool) = app_handle.try_state::<DbusPool>() {
        if let Some(conn) = pool.system().await {
            return Some(conn);
        }
    }
    // Fallback: create a new connection (shouldn't normally happen)
    Connection::system().await.ok()
}

// Public Helpers (Commands)
pub async fn has_battery() -> bool {
    match get_battery_info().await {
        Some(info) => info.has_battery,
        None => false,
    }
}

/// Public function used by commands - attempts to get battery info.
/// Creates its own system connection as a fallback if DbusPool is not accessible.
pub async fn get_battery_info() -> Option<BatteryInfo> {
    let conn = Connection::system().await.ok()?;
    get_battery_info_with_conn(&conn).await
}

/// Public function used by batch commands that have access to AppHandle.
/// Uses DbusPool for connection sharing.
pub async fn get_battery_info_with_app(app_handle: &AppHandle) -> Option<BatteryInfo> {
    let conn = get_system_connection(app_handle).await?;
    get_battery_info_with_conn(&conn).await
}

/// Internal: fetch battery info using a provided connection.
/// Uses GetAll for a single D-Bus round-trip with 300ms timeout.
/// On timeout with cache: returns cached state.
/// On initial timeout without cache: returns has_battery=false (caller should retry).
async fn get_battery_info_with_conn(conn: &Connection) -> Option<BatteryInfo> {
    // Get or discover device path
    let device_path = {
        let cached = BATTERY_DEVICE_PATH.lock().unwrap().clone();
        match cached {
            Some(p) => p,
            None => {
                match find_battery_path(conn).await {
                    Some(path) => {
                        BATTERY_DEVICE_PATH.lock().unwrap().replace(path.clone());
                        path
                    }
                    None => {
                        // Fallback path for THIS call only; do NOT cache it so
                        // subsequent calls retry discovery and can detect
                        // hot-plugged or differently named batteries.
                        "/org/freedesktop/UPower/devices/battery_BAT0".to_string()
                    }
                }
            }
        }
    };

    // Create a proxy on org.freedesktop.DBus.Properties interface for GetAll
    let props_proxy = zbus::Proxy::new(
        conn,
        "org.freedesktop.UPower",
        device_path.as_str(),
        "org.freedesktop.DBus.Properties",
    )
    .await
    .ok()?;

    // Single GetAll call with 300ms timeout
    let result = tokio::time::timeout(
        Duration::from_millis(300),
        props_proxy.call_method("GetAll", &("org.freedesktop.UPower.Device",)),
    )
    .await;

    match result {
        Ok(Ok(reply)) => {
            // Parse the HashMap from the reply body
            let props: HashMap<String, OwnedValue> = match reply.body().deserialize() {
                Ok(p) => p,
                Err(_) => {
                    // Parse failure: return cache if available
                    return BATTERY_CACHE.lock().unwrap().clone();
                }
            };

            let info = parse_battery_info_from_props(&props);
            // Update cache
            *BATTERY_CACHE.lock().unwrap() = Some(info.clone());
            Some(info)
        }
        Ok(Err(_dbus_err)) => {
            // D-Bus method call error: return cache if available
            BATTERY_CACHE.lock().unwrap().clone()
        }
        Err(_timeout) => {
            // Timeout: return cached state if available
            let cached = BATTERY_CACHE.lock().unwrap().clone();
            if cached.is_some() {
                return cached;
            }
            // Initial timeout without cache: emit has_battery: false
            // The caller (applet loop) will retry in 5s
            Some(BatteryInfo {
                has_battery: false,
                percentage: 0.0,
                state: "Unknown".to_string(),
                time_to_empty: None,
                time_to_full: None,
                is_present: false,
                is_charging: false,
                vendor: None,
                model: None,
                technology: None,
                energy: None,
                energy_full: None,
                energy_full_design: None,
                voltage: None,
                temperature: None,
                serial: None,
            })
        }
    }
}

/// Parse BatteryInfo from a HashMap of properties returned by GetAll.
fn parse_battery_info_from_props(props: &HashMap<String, OwnedValue>) -> BatteryInfo {
    let is_present = prop_bool(props, "IsPresent").unwrap_or(false);
    let percentage = prop_f64(props, "Percentage").unwrap_or(0.0);
    let state_num = prop_u32(props, "State").unwrap_or(0);
    let state_str = match state_num {
        1 => "Charging",
        2 => "Discharging",
        3 => "Empty",
        4 => "FullyCharged",
        5 => "PendingCharge",
        6 => "PendingDischarge",
        _ => "Unknown",
    }
    .to_string();
    let is_charging = state_num == 1;

    let technology_num = prop_u32(props, "Technology");
    let technology = technology_num.map(|t| match t {
        1 => "Lithium ion".to_string(),
        2 => "Lithium polymer".to_string(),
        3 => "Lithium iron phosphate".to_string(),
        4 => "Lead acid".to_string(),
        5 => "Nickel cadmium".to_string(),
        6 => "Nickel metal hydride".to_string(),
        _ => "Unknown".to_string(),
    });

    BatteryInfo {
        has_battery: is_present,
        percentage,
        state: state_str,
        time_to_empty: prop_i64(props, "TimeToEmpty").map(|v| v as u64),
        time_to_full: prop_i64(props, "TimeToFull").map(|v| v as u64),
        is_present,
        is_charging,
        vendor: prop_string(props, "Vendor"),
        model: prop_string(props, "Model"),
        technology,
        energy: prop_f64(props, "Energy"),
        energy_full: prop_f64(props, "EnergyFull"),
        energy_full_design: prop_f64(props, "EnergyFullDesign"),
        voltage: prop_f64(props, "Voltage"),
        temperature: prop_f64(props, "Temperature"),
        serial: prop_string(props, "Serial"),
    }
}

/// Merge changed properties from a PropertiesChanged signal into a cached BatteryInfo.
/// Reuses the existing prop_* helpers to parse individual OwnedValues.
fn merge_properties_into_cache(cached: &mut BatteryInfo, changed: &HashMap<String, OwnedValue>) {
    if let Some(val) = prop_bool(changed, "IsPresent") {
        cached.is_present = val;
        cached.has_battery = val;
    }

    if let Some(val) = prop_f64(changed, "Percentage") {
        cached.percentage = val;
    }

    if let Some(val) = prop_u32(changed, "State") {
        let state_str = match val {
            1 => "Charging",
            2 => "Discharging",
            3 => "Empty",
            4 => "FullyCharged",
            5 => "PendingCharge",
            6 => "PendingDischarge",
            _ => "Unknown",
        };
        cached.state = state_str.to_string();
        cached.is_charging = val == 1;
    }

    if let Some(val) = prop_i64(changed, "TimeToEmpty") {
        cached.time_to_empty = Some(val as u64);
    }

    if let Some(val) = prop_i64(changed, "TimeToFull") {
        cached.time_to_full = Some(val as u64);
    }

    if changed.contains_key("Vendor") {
        cached.vendor = prop_string(changed, "Vendor");
    }

    if changed.contains_key("Model") {
        cached.model = prop_string(changed, "Model");
    }

    if changed.contains_key("Technology") {
        cached.technology = prop_u32(changed, "Technology").map(|t| match t {
            1 => "Lithium ion".to_string(),
            2 => "Lithium polymer".to_string(),
            3 => "Lithium iron phosphate".to_string(),
            4 => "Lead acid".to_string(),
            5 => "Nickel cadmium".to_string(),
            6 => "Nickel metal hydride".to_string(),
            _ => "Unknown".to_string(),
        });
    }

    if let Some(val) = prop_f64(changed, "Energy") {
        cached.energy = Some(val);
    }

    if let Some(val) = prop_f64(changed, "EnergyFull") {
        cached.energy_full = Some(val);
    }

    if let Some(val) = prop_f64(changed, "EnergyFullDesign") {
        cached.energy_full_design = Some(val);
    }

    if let Some(val) = prop_f64(changed, "Voltage") {
        cached.voltage = Some(val);
    }

    if let Some(val) = prop_f64(changed, "Temperature") {
        cached.temperature = Some(val);
    }

    if changed.contains_key("Serial") {
        cached.serial = prop_string(changed, "Serial");
    }
}

// ---- Property extraction helpers ----

fn prop_bool(props: &HashMap<String, OwnedValue>, key: &str) -> Option<bool> {
    props.get(key).and_then(|v| <bool>::try_from(v).ok())
}

fn prop_f64(props: &HashMap<String, OwnedValue>, key: &str) -> Option<f64> {
    props.get(key).and_then(|v| <f64>::try_from(v).ok())
}

fn prop_u32(props: &HashMap<String, OwnedValue>, key: &str) -> Option<u32> {
    props.get(key).and_then(|v| <u32>::try_from(v).ok())
}

fn prop_i64(props: &HashMap<String, OwnedValue>, key: &str) -> Option<i64> {
    props.get(key).and_then(|v| <i64>::try_from(v).ok())
}

fn prop_string(props: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    props.get(key).and_then(|v| {
        // In zbus v4, &str can be extracted from &OwnedValue via TryFrom
        <&str>::try_from(v).ok().and_then(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
    })
}

/// Find the battery device path from UPower's enumerated devices.
async fn find_battery_path(conn: &Connection) -> Option<String> {
    let upower_proxy = zbus::Proxy::new(
        conn,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower",
        "org.freedesktop.UPower",
    )
    .await
    .ok()?;

    let devices: Vec<zbus::zvariant::OwnedObjectPath> = tokio::time::timeout(
        Duration::from_millis(500),
        upower_proxy.call_method("EnumerateDevices", &()),
    )
    .await
    .ok()?
    .ok()?
    .body()
    .deserialize()
    .ok()?;

    for device_path in devices {
        // Use GetAll on each device to check the Type property efficiently
        let props_proxy = zbus::Proxy::new(
            conn,
            "org.freedesktop.UPower",
            device_path.as_str(),
            "org.freedesktop.DBus.Properties",
        )
        .await
        .ok()?;

        let reply = tokio::time::timeout(
            Duration::from_millis(300),
            props_proxy.call_method("Get", &("org.freedesktop.UPower.Device", "Type")),
        )
        .await
        .ok()?
        .ok()?;

        let device_type: zbus::zvariant::OwnedValue = reply.body().deserialize().ok()?;
        if let Ok(t) = <u32>::try_from(&device_type) {
            if t == 2 {
                return Some(device_path.to_string());
            }
        }
    }
    None
}
