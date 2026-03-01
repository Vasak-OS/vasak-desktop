import { invoke } from '@tauri-apps/api/core';

export const getAudioDevices = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_audio_devices', args);
};

export const setAudioDevice = <T = any>(args: any): Promise<T> => {
	return invoke<T>('set_audio_device', args);
};

export const getBrightnessInfo = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_brightness_info', args);
};

export const setBrightnessInfo = <T = any>(args: any): Promise<T> => {
	return invoke<T>('set_brightness_info', args);
};

export const musicNowPlaying = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('music_now_playing', args);
};

export const toggleAudioMute = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_audio_mute', args);
};

export const getBatteryInfo = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_battery_info', args);
};

export const batteryExists = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('battery_exists', args);
};

export const readDirectory = <T = any>(args: any): Promise<T> => {
	return invoke<T>('read_directory', args);
};

export const logFromFrontend = <T = any>(args: any): Promise<T> => {
	return invoke<T>('log_from_frontend', args);
};

export const getLogFilePath = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_log_file_path', args);
};

export const readLogFile = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('read_log_file', args);
};

export const getLastLogLines = <T = any>(args: any): Promise<T> => {
	return invoke<T>('get_last_log_lines', args);
};

export const getSystemInfo = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_system_info', args);
};

export const globalSearch = <T = any>(args: any): Promise<T> => {
	return invoke<T>('global_search', args);
};

export const getGtkThemes = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_gtk_themes', args);
};

export const getCursorThemes = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_cursor_themes', args);
};

export const getIconPacks = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_icon_packs', args);
};

export const getShortcuts = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_shortcuts', args);
};

export const checkShortcutConflicts = <T = any>(args: any): Promise<T> => {
	return invoke<T>('check_shortcut_conflicts', args);
};

export const addCustomShortcut = <T = any>(args: any): Promise<T> => {
	return invoke<T>('add_custom_shortcut', args);
};

export const getCurrentSystemState = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_current_system_state', args);
};
