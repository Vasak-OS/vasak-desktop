import { invoke } from '@tauri-apps/api/core';

export const getAudioVolume = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_audio_volume', args);
};

export const setAudioVolume = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('set_audio_volume', args);
};

export const getAudioDevices = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_audio_devices', args);
};

export const setAudioDevice = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('set_audio_device', args);
};

export const musicNowPlaying = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('music_now_playing', args);
};
