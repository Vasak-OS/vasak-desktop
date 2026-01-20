import { invoke } from '@tauri-apps/api/core';

export const toggleNetworkApplet = async () => {
	await invoke('toggle_network_applet');
};
