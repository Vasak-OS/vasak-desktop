#!/usr/bin/env python3
"""Un org.vasak.Equalizer de prueba, para correr contra un bus privado.

Hace lo que el README de vasak-wireplumber-modules dice del servicio: las
propiedades, los cuatro métodos y PropertiesChanged con lo que cambió. No toca
el audio. Se usa con `dbus-run-session`; nunca contra el bus de la sesión.
"""
from gi.repository import Gio, GLib

XML = """
<node>
  <interface name="org.vasak.Equalizer1">
    <method name="SetGain"><arg type="u" direction="in"/><arg type="d" direction="in"/></method>
    <method name="SetGains"><arg type="ad" direction="in"/></method>
    <method name="SetPreset"><arg type="s" direction="in"/></method>
    <method name="SetEnabled"><arg type="b" direction="in"/></method>
    <property name="Frequencies" type="ad" access="read"/>
    <property name="GainRange" type="(dd)" access="read"/>
    <property name="Presets" type="as" access="read"/>
    <property name="Preset" type="s" access="read"/>
    <property name="Gains" type="ad" access="read"/>
    <property name="CustomGains" type="ad" access="read"/>
    <property name="Preamp" type="d" access="read"/>
    <property name="Enabled" type="b" access="read"/>
    <property name="Available" type="b" access="read"/>
    <property name="Saved" type="b" access="read"/>
  </interface>
</node>
"""
PRESETS = {
    "flat": [0.0] * 10,
    "bass": [6, 5, 4, 2, 0, 0, 0, 0, 0, 0],
    "treble": [0, 0, 0, 0, 0, 1, 2, 4, 5, 6],
    "vocal": [-2, -1, 0, 2, 4, 4, 3, 1, 0, -1],
    "pop": [-1, 1, 3, 4, 3, 1, -1, -1, -1, -1],
    "rock": [4.5, 3.5, 1.5, -1, -2, -1, 1.5, 3, 4, 4.5],
    "jazz": [3, 2, 1, 2, -1, -1, 0, 1, 2, 3],
    "classic": [4, 3, 2, 1, -1, -1, 0, 2, 3, 4],
}
state = {"Preset": "flat", "Custom": [0.0] * 10, "Enabled": True}


def gains():
    return state["Custom"] if state["Preset"] == "custom" else [float(g) for g in PRESETS[state["Preset"]]]


def prop(name):
    return {
        "Frequencies": GLib.Variant("ad", [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]),
        "GainRange": GLib.Variant("(dd)", (-12.0, 12.0)),
        "Presets": GLib.Variant("as", list(PRESETS)),
        "Preset": GLib.Variant("s", state["Preset"]),
        "Gains": GLib.Variant("ad", gains()),
        "CustomGains": GLib.Variant("ad", state["Custom"]),
        "Preamp": GLib.Variant("d", 0.0),
        "Enabled": GLib.Variant("b", state["Enabled"]),
        "Available": GLib.Variant("b", True),
        "Saved": GLib.Variant("b", True),
    }[name]


def changed(conn, names):
    conn.emit_signal(None, "/org/vasak/Equalizer", "org.freedesktop.DBus.Properties", "PropertiesChanged",
                     GLib.Variant("(sa{sv}as)", ("org.vasak.Equalizer1", {n: prop(n) for n in names}, [])))


def on_call(conn, sender, path, iface, method, params, invocation):
    args = params.unpack()
    if method == "SetGain":
        band, gain = args
        if band > 9 or not -12 <= gain <= 12:
            invocation.return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", "fuera de rango")
            return
        state["Custom"] = list(gains())
        state["Custom"][band] = gain
        state["Preset"] = "custom"
    elif method == "SetGains":
        state["Custom"] = list(args[0])
        state["Preset"] = "custom"
    elif method == "SetPreset":
        if args[0] not in PRESETS and args[0] != "custom":
            invocation.return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", "perfil desconocido")
            return
        state["Preset"] = args[0]
    elif method == "SetEnabled":
        state["Enabled"] = args[0]
    invocation.return_value(None)
    changed(conn, ["Preset", "Gains", "CustomGains", "Enabled"])


def on_get(conn, sender, path, iface, name):
    return prop(name)


def on_bus(conn, name):
    node = Gio.DBusNodeInfo.new_for_xml(XML)
    conn.register_object("/org/vasak/Equalizer", node.interfaces[0], on_call, on_get, None)


Gio.bus_own_name(Gio.BusType.SESSION, "org.vasak.Equalizer", Gio.BusNameOwnerFlags.NONE, on_bus, None, None)
GLib.MainLoop().run()
