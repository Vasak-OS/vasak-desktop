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

export const openFileManagerWindow = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('open_file_manager_window', args);
};

export const openConfigurationWindow = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('open_configuration_window', args);
};

export const toggleMenu = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_menu', args);
};

export const toggleConfigApp = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_config_app', args);
};
