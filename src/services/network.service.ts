import { invoke } from '@tauri-apps/api/core';

export const isWirelessAvailable = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('plugin:network-manager|is_wireless_available', args);
};

export const getWirelessEnabled = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('plugin:network-manager|get_wireless_enabled', args);
};

export const setWirelessEnabled = <T = any>(args: any): Promise<T> => {
	return invoke<T>('plugin:network-manager|set_wireless_enabled', args);
};

export const toggleNetworkApplet = async () => {
	await invoke('toggle_network_applet');
};