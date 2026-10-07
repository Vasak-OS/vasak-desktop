/**
 * El micrófono y las entradas de audio (vasak-desktop#182).
 *
 * Aparte de `core.service.ts` porque una prueba dobla ese módulo entero con
 * `mock.module`, y el doble queda en la caché para las demás.
 */
import { invoke } from '@tauri-apps/api/core';
import type { AudioDevice } from '@/interfaces/audio-device';
import type { VolumeInfo } from '@/interfaces/volume';

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
