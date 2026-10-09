/**
 * Los `@/…` de `MicrophoneControl.vue`, `VolumeControl.vue` y
 * `AudioDeviceSelector.vue` para montarlos en las pruebas
 * (vasak-desktop#182). Lo que se dobla es el `invoke` de Tauri y los eventos,
 * desde la prueba; el micrófono, las entradas y el bus son los de verdad.
 *
 * Lo que viene de `core.service.ts` se escribe acá con el mismo `invoke`: otra
 * prueba dobla ese módulo entero con `mock.module`, y el doble queda en la
 * caché según el orden en que corran los archivos. Y el logger tampoco se
 * carga: construirlo antes que esa prueba le gana a su doble.
 */
import { invoke } from '@tauri-apps/api/core';
import { computed, ref } from 'vue';
import type { VolumeInfo } from '../../src/interfaces/volume';

export {
	getAudioInputDevices,
	setAudioInputDevice,
} from '../../src/services/audio-input.service';
export { useMicrophoneState } from '../../src/tools/composables/useMicrophoneState';
export { useSharedEvent } from '../../src/tools/event.bus';

export const getAudioDevices = <T = unknown>(): Promise<T> => invoke<T>('get_audio_devices');
export const setAudioDevice = <T = unknown>(args: { deviceId: string }): Promise<T> =>
	invoke<T>('set_audio_device', args);

/** El volumen de salida, lo justo para `VolumeControl`: lee y cambia por `invoke`. */
export function useVolumeState() {
	const volumeInfo = ref<VolumeInfo>({ current: 0, min: 0, max: 100, is_muted: false });
	const currentVolume = ref(0);
	return {
		volumeInfo,
		currentVolume,
		currentIcon: computed(() => 'audio-volume-medium-symbolic'),
		async getVolumeInfo() {
			volumeInfo.value = await invoke<VolumeInfo>('get_audio_volume');
			currentVolume.value = volumeInfo.value.current;
		},
		updateVolume() {},
		async toggleMute() {},
		getPercentageClass: () => '',
	};
}

/** El de verdad reemplaza la consola al cargarse: acá alcanza con decirlo. */
export const logError = (message: string, data?: unknown) => console.error(message, data);
