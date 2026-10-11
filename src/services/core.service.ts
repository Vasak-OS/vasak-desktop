import { invoke } from '@tauri-apps/api/core';

export const getAudioDevices = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_audio_devices', args);
};

export const setAudioDevice = <T = any>(args: any): Promise<T> => {
	return invoke<T>('set_audio_device', args);
};

/**
 * Las salidas, incluyendo las disponibles-pero-inactivas: perfiles de `card`
 * sin sink todavía, como el HDMI de la placa en otro perfil (vasak-desktop#230).
 */
export const getAudioOutputs = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_audio_outputs', args);
};

/**
 * Elige una salida. Con `sinkId` fija un sink existente; con `card` + `profile`
 * conmuta el perfil de la placa (crea el sink) y lo fija.
 */
export const setAudioOutput = <T = any>(args: {
	sinkId?: string;
	card?: string | null;
	profile?: string | null;
}): Promise<T> => {
	return invoke<T>('set_audio_output', args);
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
	return invoke<T>('privacy_in_use', args);
};

/**
 * Corta una captura de pantalla en curso.
 *
 * Lo único de los tres que se puede retirar: la cámara y el micrófono los abre
 * la aplicación contra el dispositivo y no hay nada en el medio que pueda
 * quitárselos.
 */
export const privacyStopScreen = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('privacy_stop_screen', args);
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

export const getAudioVolume = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_audio_volume', args);
};

export const setAudioVolume = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('set_audio_volume', args);
};
