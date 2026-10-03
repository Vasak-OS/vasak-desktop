import { invoke } from '@tauri-apps/api/core';
import { onUnmounted, ref } from 'vue';
import { type EqualizerState, missingEqualizer } from '@/interfaces/equalizer';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';
import { rateLimited } from '@/utils/rate-limit';

/**
 * El ecualizador de sistema (`org.vasak.Equalizer1`), para el reproductor
 * desplegable.
 *
 * El estado llega entero por `equalizer-changed` cada vez que el servicio
 * cambia, aparece o se va (`applets/equalizer.rs`). Arrastrar una banda la
 * mueve acá en el acto —la curva sigue a la mano— y manda `SetGain` como mucho
 * unas 30 veces por segundo, siempre con el último valor; el servicio guarda
 * solo medio segundo después. Elegir un perfil lo pide y espera los `Gains`
 * que llegan, hacia los que se anima la curva.
 */
export const GAIN_INTERVAL_MS = 34;

export function useEqualizer() {
	const state = ref<EqualizerState>(missingEqualizer());

	async function load(): Promise<void> {
		try {
			state.value = await invoke<EqualizerState>('equalizer_state');
		} catch (error) {
			logError('[equalizer] no se pudo leer el estado:', error);
			state.value = missingEqualizer();
		}
	}

	useSharedEvent<EqualizerState>('equalizer-changed', (payload) => {
		if (payload) state.value = payload;
	});

	const gains = rateLimited<number, number>((band, gain) => {
		invoke('equalizer_set_gain', { band, gain }).catch((error) =>
			logError('[equalizer] no se pudo mover la banda:', error)
		);
	}, GAIN_INTERVAL_MS);

	function setGain(band: number, gain: number): void {
		const next = [...state.value.gains];
		next[band] = gain;
		state.value = { ...state.value, gains: next, preset: 'custom', saved: false };
		gains.push(band, gain);
	}

	async function setPreset(preset: string): Promise<void> {
		try {
			await invoke('equalizer_set_preset', { preset });
		} catch (error) {
			logError('[equalizer] no se pudo elegir el perfil:', error);
		}
	}

	onUnmounted(() => gains.flush());

	return { state, load, setGain, setPreset };
}
