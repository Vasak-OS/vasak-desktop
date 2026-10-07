import { invoke } from '@tauri-apps/api/core';
import type { AudioDevice } from '@/interfaces/audio-device';
import type { VolumeInfo } from '@/interfaces/volume';

export const getAudioDevices = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_audio_devices', args);
};

export const setAudioDevice = <T = any>(args: any): Promise<T> => {
	return invoke<T>('set_audio_device', args);
};

// ── El micrófono (vasak-desktop#182) ─────────────────────────────────────────

/** El volumen y el silencio de la fuente por omisión, o `null` sin micrófono. */
export const getMicrophone = (): Promise<VolumeInfo | null> => invoke('get_microphone');

export const setMicrophoneVolume = (volume: number): Promise<void> =>
	invoke('set_microphone_volume', { volume });

/** Devuelve si el micrófono quedó silenciado. */
export const toggleMicrophoneMute = (): Promise<boolean> => invoke('toggle_microphone_mute');

/** Las entradas de audio; el `id` es el nombre del nodo de PipeWire. */
export const getAudioInputDevices = (): Promise<AudioDevice[]> => invoke('get_audio_input_devices');

export const setAudioInputDevice = (deviceId: string): Promise<boolean> =>
	invoke('set_audio_input_device', { deviceId });

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
