import { invoke } from '@tauri-apps/api/core';
import type {
	ConnectApp,
	ConnectCamera,
	ConnectDevice,
	ConnectRunningApp,
	ConnectWebcamState,
} from '@/interfaces/connect';

/**
 * The phones connected right now.
 *
 * Never rejects: an empty list covers both "no phone" and "the service is not
 * running", which look the same from the panel and neither deserves an error.
 */
export const listConnectDevices = (): Promise<ConnectDevice[]> =>
	invoke<ConnectDevice[]>('connect_list_devices');

/**
 * The apps installed on a phone.
 *
 * The first call for a device takes a few seconds — the phone walks its whole
 * package database — and the daemon caches it afterwards. Pass `refresh` to
 * make it look again; nothing tells us when an app is installed on the phone.
 */
export const listConnectApps = (serial: string, refresh = false): Promise<ConnectApp[]> =>
	invoke<ConnectApp[]>('connect_list_apps', { serial, refresh });

/** Opens an app in its own window. Resolves to the pid of its scrcpy process. */
export const launchConnectApp = (serial: string, pkg: string): Promise<number> =>
	invoke<number>('connect_launch_app', { serial, package: pkg });

export const stopConnectApp = (serial: string, pkg: string): Promise<boolean> =>
	invoke<boolean>('connect_stop_app', { serial, package: pkg });

export const listConnectRunning = (): Promise<ConnectRunningApp[]> =>
	invoke<ConnectRunningApp[]>('connect_list_running');

export const toggleConnectMenu = (): Promise<void> => invoke<void>('toggle_connect_menu');

/** The cameras a phone has. Cached by the daemon; `refresh` re-asks the phone. */
export const listConnectCameras = (serial: string, refresh = false): Promise<ConnectCamera[]> =>
	invoke<ConnectCamera[]>('connect_list_cameras', { serial, refresh });

/**
 * Starts writing a phone camera into the loopback device.
 *
 * Resolves to the device path other applications open. An empty `size` and a
 * zero `fps` let the phone choose, which is what the control centre does — the
 * modes belong in the settings screen, where there is room for three selects.
 *
 * Rejects, unlike the listing calls: this runs because somebody pressed a
 * switch, and a switch that silently returns to off explains nothing.
 */
export const startConnectWebcam = (
	serial: string,
	cameraId: string,
	size = '',
	fps = 0
): Promise<string> => invoke<string>('connect_start_webcam', { serial, cameraId, size, fps });

/** Stops the stream. `false` means there was nothing streaming. */
export const stopConnectWebcam = (): Promise<boolean> => invoke<boolean>('connect_stop_webcam');

/** What the bridge is doing, and whether it could run at all. */
export const connectWebcamState = (): Promise<ConnectWebcamState> =>
	invoke<ConnectWebcamState>('connect_webcam_state');
