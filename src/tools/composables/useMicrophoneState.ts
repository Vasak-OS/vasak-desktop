import { computed, ref } from 'vue';
import type { VolumeInfo } from '@/interfaces/volume';
import {
	getMicrophone,
	setMicrophoneVolume,
	toggleMicrophoneMute,
} from '@/services/audio-input.service';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';
import { calculateVolumePercentage, getMicrophoneIconName } from '@/utils/volume';

/** El evento del escritorio con el micrófono nuevo (o `null` si no hay). */
export const MICROPHONE_EVENT = 'microphone-changed';

/**
 * El micrófono para un componente (vasak-desktop#182): el volumen y el
 * silencio de la fuente por omisión.
 *
 * Lee una vez al montar —de lo que el escritorio ya tiene del flujo de
 * PipeWire, sin procesos— y después lo sigue por `microphone-changed`, sin
 * sondeos. Un cambio hecho desde otra aplicación, o por elegir otra entrada,
 * llega por el mismo evento.
 */
export function useMicrophoneState() {
	/** `null` mientras no se sabe o si el equipo no tiene micrófono. */
	const microphone = ref<VolumeInfo | null>(null);
	const loaded = ref(false);
	const currentVolume = ref(0);

	const available = computed(() => microphone.value !== null);
	const isMuted = computed(() => microphone.value?.is_muted ?? false);
	const percentage = computed(() =>
		microphone.value ? calculateVolumePercentage(microphone.value, currentVolume.value) : 0
	);
	const currentIcon = computed(() => getMicrophoneIconName(isMuted.value, percentage.value));

	function apply(info: VolumeInfo | null): void {
		microphone.value = info;
		currentVolume.value = info?.current ?? 0;
		loaded.value = true;
	}

	async function refresh(): Promise<void> {
		try {
			apply(await getMicrophone());
		} catch (error) {
			logError('[microphone] no se pudo leer el micrófono:', error);
			loaded.value = true;
		}
	}

	// Como el volumen de salida: el deslizador avisa en cada movimiento y cada
	// orden es un proceso de `pactl`; va sólo el último valor de la ráfaga.
	let commitTimer: ReturnType<typeof setTimeout> | undefined;
	const COMMIT_DELAY = 60;

	function updateVolume(value: number = currentVolume.value): void {
		currentVolume.value = value;
		if (commitTimer !== undefined) clearTimeout(commitTimer);
		commitTimer = setTimeout(() => {
			commitTimer = undefined;
			setMicrophoneVolume(currentVolume.value).catch((error) => {
				logError('[microphone] no se pudo cambiar el volumen:', error);
			});
		}, COMMIT_DELAY);
	}

	async function toggleMute(): Promise<void> {
		if (!microphone.value) return;
		try {
			const muted = await toggleMicrophoneMute();
			if (microphone.value) microphone.value = { ...microphone.value, is_muted: muted };
		} catch (error) {
			logError('[microphone] no se pudo silenciar:', error);
		}
	}

	useSharedEvent<VolumeInfo | null>(MICROPHONE_EVENT, (payload) => apply(payload), {
		throttleMs: 16,
	});

	return {
		microphone,
		loaded,
		available,
		isMuted,
		currentVolume,
		percentage,
		currentIcon,
		refresh,
		updateVolume,
		toggleMute,
	};
}
