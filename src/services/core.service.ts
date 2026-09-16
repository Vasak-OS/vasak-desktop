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
	return invoke<T>('battery_fetch_info', args);
};

export const batteryExists = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('battery_exists', args);
};

/**
 * Qué está usando la cámara o el micrófono ahora mismo.
 *
 * El panel se destruye y se vuelve a crear cuando cambian los monitores, y el
 * componente nuevo nace vacío. El escritorio no repite un anuncio igual al
 * anterior, así que sin esta consulta el indicador se quedaría invisible con la
 * cámara encendida hasta el próximo cambio.
 */
export const privacyInUse = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('privacidad_en_uso', args);
};

/**
 * Corta una captura de pantalla en curso.
 *
 * Lo único de los tres que se puede retirar: la cámara y el micrófono los abre
 * la aplicación contra el dispositivo y no hay nada en el medio que pueda
 * quitárselos.
 */
export const privacyStopScreen = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('privacidad_cortar', args);
};

/** Abre o cierra el applet que lista todo y deja cortar. */
export const togglePrivacyApplet = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_privacidad_applet', args);
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

export const globalSearch = <T = any>(args: any): Promise<T> => {
	return invoke<T>('global_search', args);
};

export const getAudioVolume = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_audio_volume', args);
};

export const setAudioVolume = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('set_audio_volume', args);
};
