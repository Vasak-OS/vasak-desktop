import { invoke } from '@tauri-apps/api/core';
import type { ConnectApp, ConnectDevice, ConnectRunningApp } from '@/interfaces/connect';

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
