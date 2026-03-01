import { invoke } from '@tauri-apps/api/core';

export const detectDisplayServer = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('detect_display_server', args);
};

export const logout = <T = any>(args: any): Promise<T> => {
	return invoke<T>('logout', args);
};

export const shutdown = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('shutdown', args);
};

export const reboot = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('reboot', args);
};

export const suspend = <T = any>(args: any): Promise<T> => {
	return invoke<T>('suspend', args);
};

export const getSystemConfig = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_system_config', args);
};

export const setSystemConfig = <T = any>(args: any): Promise<T> => {
	return invoke<T>('set_system_config', args);
};
