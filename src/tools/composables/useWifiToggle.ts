import { ref } from 'vue';
import {
	getWirelessEnabled,
	isWirelessAvailable,
	setWirelessEnabled,
} from '@/services/network.service';

// Los errores van por `console.error` y no por `logError`: el logger del
// escritorio reemplaza `console.error` al arrancar y lo manda al mismo archivo,
// así que en la sesión es lo mismo. Importar el logger acá lo construiría en
// las pruebas antes que `el-logger-no-se-llama-a-si-mismo.test.ts`, que necesita
// ser el primero en hacerlo (y en CI el orden de los archivos no es fijo).

/**
 * Prender y apagar el Wi-Fi desde el mosaico del centro de control
 * (vasak-desktop#175).
 *
 * Lo mismo que hace el interruptor del panel de red, en su forma mínima: si el
 * equipo tiene radio, si está prendida y alternarla. Sin radio, el mosaico se
 * ve «no disponible» en lugar de roto.
 */
export function useWifiToggle() {
	const available = ref(true);
	const enabled = ref(false);
	const busy = ref(false);

	async function refresh(): Promise<void> {
		try {
			available.value = await isWirelessAvailable();
			enabled.value = available.value ? await getWirelessEnabled() : false;
		} catch (error) {
			console.error('[wifi] no se pudo leer el estado de la radio:', error);
		}
	}

	async function toggle(): Promise<void> {
		if (!available.value || busy.value) return;
		busy.value = true;
		const wanted = !enabled.value;
		try {
			await setWirelessEnabled(wanted);
			enabled.value = wanted;
		} catch (error) {
			console.error('[wifi] no se pudo cambiar la radio:', error);
		} finally {
			busy.value = false;
		}
	}

	return { available, enabled, busy, refresh, toggle };
}
