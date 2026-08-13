/**
 * The Android device service (`vasak-connect`).
 *
 * Mirrors the D-Bus contract published by that daemon. The service is the
 * source of truth; this only describes what arrives.
 */

/** How the daemon is reaching a phone. */
export type ConnectTransport = 'usb' | 'tcp';

/**
 * `unauthorized` is the one worth handling on its own: the cable is in but
 * nobody has tapped "Allow USB debugging" yet. It is neither an error nor a
 * working device, and treating it as either leaves the person staring at a
 * menu that never fills.
 */
export type ConnectDeviceState = 'ready' | 'unauthorized' | 'connecting' | 'offline';

export interface ConnectDevice {
	serial: string;
	model: string;
	transport: ConnectTransport;
	state: ConnectDeviceState;
	trusted: boolean;
	address: string;
}

export interface ConnectApp {
	package: string;
	label: string;
	/** Shipped with Android. Hidden from the list unless asked for. */
	system: boolean;
	/** Path to an icon, or empty — the daemon does not extract them yet. */
	icon: string;
}

export interface ConnectRunningApp {
	serial: string;
	package: string;
	label: string;
	pid: number;
}
