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

/**
 * Which way a camera points.
 *
 * The only thing that tells a person which camera they are picking: an id of
 * `0` or `2` means nothing to anybody. `external` also covers a word the
 * daemon did not recognise, so the camera is still listed — just without a
 * promise about where it points.
 */
export type ConnectCameraFacing = 'back' | 'front' | 'external';

export interface ConnectCamera {
	/** scrcpy's camera id. Opaque, and only unique within one phone. */
	id: string;
	facing: ConnectCameraFacing;
	/** Capture sizes the sensor accepts, largest first, as `1280x720`. */
	sizes: string[];
	fps: number[];
}

/** What the webcam bridge is doing. */
export interface ConnectWebcamState {
	active: boolean;
	/**
	 * The device other applications open, e.g. `/dev/video42`.
	 *
	 * Empty means the v4l2loopback module is not loaded — reported outside
	 * `active` because it is the one failure a person can fix, and they have to
	 * see it before pressing anything.
	 */
	device: string;
	/** Which phone is feeding it. Empty unless `active`. */
	serial: string;
	/** Empty unless `active`. */
	camera_id: string;
	/** Empty unless `active`. */
	size: string;
}
