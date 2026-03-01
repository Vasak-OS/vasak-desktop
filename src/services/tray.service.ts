import { invoke } from '@tauri-apps/api/core';

export const trayItemActivate = <T = any>(args: any): Promise<T> => {
	return invoke<T>('tray_item_activate', args);
};

export const trayItemSecondaryActivate = <T = any>(args: any): Promise<T> => {
	return invoke<T>('tray_item_secondary_activate', args);
};

export const getTrayMenu = <T = any>(args: any): Promise<T> => {
	return invoke<T>('get_tray_menu', args);
};

export const trayMenuItemClick = <T = any>(args: any): Promise<T> => {
	return invoke<T>('tray_menu_item_click', args);
};

export const initSniWatcher = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('init_sni_watcher', args);
};

export const getTrayItems = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_tray_items', args);
};
