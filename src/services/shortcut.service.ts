import { invoke } from '@tauri-apps/api/core';

export const updateShortcut = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('update_shortcut', args);
};

export const executeShortcut = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('execute_shortcut', args);
};

export const deleteShortcut = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('delete_shortcut', args);
};

export const getShortcuts = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_shortcuts', args);
};

export const checkShortcutConflicts = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('check_shortcut_conflicts', args);
};

export const addCustomShortcut = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('add_custom_shortcut', args);
};
