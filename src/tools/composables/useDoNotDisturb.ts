import { onMounted, readonly, ref } from 'vue';
import {
	DO_NOT_DISTURB_EVENT,
	type DoNotDisturbState,
	getDoNotDisturb,
	setDoNotDisturb,
} from '@/services/do-not-disturb.service';
import { useSharedEvent } from '@/tools/event.bus';

// Los errores van por `console.error` y no por `logError`: el logger del
// escritorio reemplaza `console.error` al arrancar y lo manda al mismo archivo,
// así que en la sesión es lo mismo (ver `useWifiToggle`).

/**
 * «No molestar» para un componente: el mosaico, el botón redondo y el
 * indicador de la bandeja (vasak-desktop#177).
 *
 * Lee el estado una vez al montar —de la copia en memoria del escritorio, sin
 * cruzar el bus— y después lo sigue por el evento, sin sondeos. Un cambio hecho
 * desde otra ventana o por el modo juego llega por el mismo evento.
 */
export function useDoNotDisturb() {
	const available = ref(false);
	const enabled = ref(false);
	const busy = ref(false);

	function apply(state: DoNotDisturbState): void {
		available.value = state.available;
		enabled.value = state.available && state.enabled;
	}

	onMounted(async () => {
		try {
			apply(await getDoNotDisturb());
		} catch (error) {
			console.error('[do-not-disturb] no se pudo leer el estado:', error);
		}
	});

	useSharedEvent<DoNotDisturbState>(DO_NOT_DISTURB_EVENT, apply);

	/** Lo alterna. Devuelve el estado anterior, o `null` si no se pudo. */
	async function toggle(): Promise<boolean | null> {
		if (!available.value || busy.value) return null;
		busy.value = true;
		const wanted = !enabled.value;
		try {
			const previous = await setDoNotDisturb(wanted);
			enabled.value = wanted;
			return previous;
		} catch (error) {
			console.error('[do-not-disturb] no se pudo cambiar el modo:', error);
			return null;
		} finally {
			busy.value = false;
		}
	}

	return {
		available: readonly(available),
		enabled: readonly(enabled),
		busy: readonly(busy),
		toggle,
	};
}
