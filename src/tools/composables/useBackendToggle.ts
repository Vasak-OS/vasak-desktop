import { onMounted, readonly, ref } from 'vue';
import { useSharedEvent } from '@/tools/event.bus';

// Los errores van por `console.error` y no por `logError`: el logger del
// escritorio reemplaza `console.error` al arrancar y lo manda al mismo archivo,
// así que en la sesión es lo mismo (ver `useWifiToggle`).

/** Lo que el componible necesita saber del estado del ajuste. */
export interface ToggleState {
	/** Falso si quien lo sostiene no está o no lo entiende. */
	available: boolean;
	enabled: boolean;
}

export interface BackendToggleOptions<T> {
	/** Para los mensajes del registro: `[name] no se pudo…`. */
	name: string;
	/** El evento que llega a cada ventana cuando el ajuste cambia. */
	event: string;
	/** La lectura inicial, una sola vez al montar. */
	read: () => Promise<T>;
	/** Lo pone y devuelve el estado anterior; rechaza si no pudo. */
	write: (enabled: boolean) => Promise<boolean>;
	/** De lo que contesta `read` (y trae el evento) al estado. */
	toState: (payload: T) => ToggleState;
}

/**
 * Un ajuste de encendido y apagado que vive en el escritorio —No molestar
 * (vasak-desktop#177), el modo juego (#181)— visto desde un componente.
 *
 * Lee el estado una vez al montar y después lo sigue por el evento, sin
 * sondeos: un cambio hecho desde otra ventana llega por el mismo camino. Si
 * cambiarlo falla, el estado queda en lo que de verdad hay y `failed` lo dice
 * hasta el próximo cambio que salga bien.
 */
export function useBackendToggle<T>(options: BackendToggleOptions<T>) {
	const available = ref(false);
	const enabled = ref(false);
	const busy = ref(false);
	const failed = ref(false);
	/**
	 * Cuántas veces cambió el estado desde que se montó. La lectura inicial
	 * puede volver después de un evento o de un cambio propio: si llegó algo
	 * más nuevo mientras tanto, la respuesta vieja se descarta.
	 */
	let revision = 0;

	function apply(payload: T): void {
		const state = options.toState(payload);
		revision += 1;
		available.value = state.available;
		enabled.value = state.available && state.enabled;
		failed.value = false;
	}

	onMounted(async () => {
		const asked = revision;
		try {
			const payload = await options.read();
			if (revision === asked) apply(payload);
		} catch (error) {
			console.error(`[${options.name}] no se pudo leer el estado:`, error);
		}
	});

	useSharedEvent<T>(options.event, apply);

	/** Lo alterna. Devuelve el estado anterior, o `null` si no se pudo. */
	async function toggle(): Promise<boolean | null> {
		if (!available.value || busy.value) return null;
		busy.value = true;
		const wanted = !enabled.value;
		try {
			const previous = await options.write(wanted);
			revision += 1;
			enabled.value = wanted;
			failed.value = false;
			return previous;
		} catch (error) {
			console.error(`[${options.name}] no se pudo cambiar:`, error);
			failed.value = true;
			return null;
		} finally {
			busy.value = false;
		}
	}

	return {
		available: readonly(available),
		enabled: readonly(enabled),
		busy: readonly(busy),
		failed: readonly(failed),
		toggle,
	};
}
