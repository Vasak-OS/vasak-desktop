import {
	getPowerState,
	onPowerStateChanged,
	type PowerState,
	setPowerProfile,
} from '@vasakgroup/plugin-power-profiles';
import { onBeforeUnmount, onMounted, ref, shallowRef } from 'vue';

// Los errores van por `console.error` y no por `logError`: el logger del
// escritorio reemplaza `console.error` al arrancar y lo manda al mismo archivo,
// así que en la sesión es lo mismo (ver `useBackendToggle`). Importar el logger
// acá lo construiría en las pruebas antes de que la suya lo doble.

const UNAVAILABLE: PowerState = {
	available: false,
	profiles: [],
	activeProfile: null,
	performanceDegraded: null,
};

/**
 * El perfil de energía para el centro de control (vasak-desktop#189), del
 * plugin `power-profiles`.
 *
 * Una lectura al montar —que no va al bus: el plugin guarda una copia— y
 * después la señal del demonio, reenviada como evento: si el perfil cambia
 * desde Configuración, desde otra aplicación o porque el demonio lo limitó, se
 * ve sin volver a preguntar. Sin power-profiles-daemon queda no disponible.
 */
export function usePowerProfile() {
	const state = shallowRef<PowerState>(UNAVAILABLE);
	const loaded = ref(false);
	let unlisten: (() => void) | null = null;
	let disposed = false;

	function apply(next: PowerState): void {
		state.value = next;
	}

	onMounted(async () => {
		try {
			apply(await getPowerState());
		} catch (error) {
			console.error('[power-profile] no se pudo leer el perfil:', error);
		} finally {
			loaded.value = true;
		}
		try {
			const stop = await onPowerStateChanged(apply);
			// Si el componente se fue mientras se registraba, se suelta ya.
			if (disposed) stop();
			else unlisten = stop;
		} catch (error) {
			console.error('[power-profile] no se pudo escuchar el perfil:', error);
		}
	});

	onBeforeUnmount(() => {
		disposed = true;
		unlisten?.();
		unlisten = null;
	});

	/**
	 * Cambia el perfil. Si el demonio lo rechaza, vuelve a mostrar el que
	 * estaba: el selector ya se había movido.
	 */
	async function choose(profile: string): Promise<void> {
		const previous = state.value;
		if (!previous.available || profile === previous.activeProfile) return;
		apply({ ...previous, activeProfile: profile });
		try {
			apply(await setPowerProfile(profile));
		} catch (error) {
			console.error('[power-profile] no se pudo cambiar el perfil:', error);
			apply(previous);
		}
	}

	return { state, loaded, choose };
}
