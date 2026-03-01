import { invoke } from '@tauri-apps/api/core';

export const openApp = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('open_app', args);
};

export const getMenuItems = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_menu_items', args);
};
