import { invoke } from '@tauri-apps/api/core';

export const toggleNetworkApplet = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_network_applet', args);
};

export const getWindows = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_windows', args);
};

export const toggleBluetoothApplet = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_bluetooth_applet', args);
};

export const toggleAudioApplet = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_audio_applet', args);
};

export const toggleWindow = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_window', args);
};

export const toggleSearch = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_search', args);
};

export const toggleControlCenter = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_control_center', args);
};

/** Launches the separate vasak-settings application. */
export const openSettings = <T = any>(): Promise<T> => {
	return invoke<T>('open_settings');
};

export const toggleMenu = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_menu', args);
};

export const toggleSessionPopup = <T = any>(action: string): Promise<T> => {
	return invoke<T>('toggle_session_popup', { action });
};
