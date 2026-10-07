import {
	type BrightnessReport,
	getBrightness,
	type MonitorBrightness,
	onBrightnessChanged,
	setBrightness,
} from '@vasakgroup/plugin-display-manager';
import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue';
import { createLatestWriter, monitorKey, primaryMonitor } from '@/tools/display-brightness';
import { eventBus } from '@/tools/event.bus';

// Los errores van por `console.error` y no por `logError`: el logger del
// escritorio reemplaza `console.error` al arrancar y lo manda al mismo archivo,
// así que en la sesión es lo mismo (ver `useBackendToggle`). Importar el logger
// acá lo construiría en las pruebas antes de que la suya lo doble.

/**
 * El brillo de las pantallas para el centro de control (vasak-desktop#189),
 * del plugin `display-manager`.
 *
 * Un solo estado para la ventana: el deslizador del estado A (el monitor
 * principal) y la lista del estado B (uno por monitor) leen el mismo informe,
 * con una sola lectura y un solo oyente del evento, por más componentes que lo
 * usen. Al irse el último, se suelta el oyente.
 *
 * - **Sin sondeos.** Los cambios llegan por `display-brightness-changed`: el
 *   kernel avisa cuando cambia el brillo del panel (también por una tecla) o
 *   cuando se conecta un monitor, y el plugin cuando termina de leer uno
 *   externo.
 * - **DDC/CI nunca en el camino de abrir.** El plugin busca los monitores
 *   externos al iniciar la sesión (`prefetch_ddc`) y `getBrightness` devuelve
 *   lo guardado, sin esperar. Al volver a abrir el centro, si hay monitores
 *   externos, se pregunta otra vez: el plugin relee en segundo plano los que
 *   tengan más de un minuto —los botones del propio monitor no avisan— y lo
 *   nuevo llega por el evento.
 * - **Arrastrar no escribe.** Mientras se arrastra, el valor es un borrador
 *   local; se escribe al soltar (`commit`), y si todavía se está escribiendo
 *   el anterior, sólo el último (`createLatestWriter`).
 */

const report = shallowRef<BrightnessReport | null>(null);
/** Lo que se está arrastrando o escribiendo, por monitor: gana sobre el informe. */
const drafts = ref<Record<string, number>>({});

let users = 0;
let stopListening: (() => void) | null = null;
let stopShown: (() => void) | null = null;

function apply(next: BrightnessReport): void {
	report.value = next;
}

/**
 * Lo escrito ya es el valor del monitor. El aviso del panel llega por el
 * uevent poco después; sin esto el deslizador saltaría al valor viejo en el
 * medio.
 */
function patch(monitor: MonitorBrightness, percent: number): void {
	const current = report.value;
	if (!current) return;
	const key = monitorKey(monitor);
	report.value = {
		...current,
		monitors: current.monitors.map((m) => (monitorKey(m) === key ? { ...m, percent } : m)),
	};
}

const write = createLatestWriter<{ monitor: MonitorBrightness; percent: number }>(
	async (_key, { monitor, percent }) => {
		await setBrightness(monitor.kind, monitor.handle, percent);
		patch(monitor, percent);
	}
);

async function load(): Promise<void> {
	try {
		apply(await getBrightness());
	} catch (error) {
		console.error('[brightness] no se pudo leer el brillo:', error);
	}
}

/** Al volver a abrir el centro: sólo si hay monitores externos que releer. */
function onShown(): void {
	if (report.value?.monitors.some((m) => m.kind === 'ddc')) void load();
}

async function subscribe(): Promise<void> {
	stopShown = eventBus.subscribe('window-shown', onShown);
	await load();
	try {
		const unlisten = await onBrightnessChanged(apply);
		// Si el último componente se fue mientras se registraba, se suelta ya.
		if (users === 0) unlisten();
		else stopListening = unlisten;
	} catch (error) {
		console.error('[brightness] no se pudo escuchar el brillo:', error);
	}
}

function unsubscribe(): void {
	stopListening?.();
	stopListening = null;
	stopShown?.();
	stopShown = null;
}

export function useDisplayBrightness() {
	onMounted(() => {
		users += 1;
		if (users === 1) void subscribe();
	});

	onBeforeUnmount(() => {
		users -= 1;
		if (users === 0) unsubscribe();
	});

	const monitors = computed(() => report.value?.monitors ?? []);
	const primary = computed(() => primaryMonitor(report.value));
	const ddc = computed(() => report.value?.ddc ?? null);
	const loaded = computed(() => report.value !== null);

	/** Lo que muestra el deslizador: el borrador si hay uno, y si no, el informe. */
	function percentOf(monitor: MonitorBrightness): number {
		return drafts.value[monitorKey(monitor)] ?? monitor.percent;
	}

	/** Arrastrando: se mueve el deslizador y no se escribe nada. */
	function preview(monitor: MonitorBrightness, percent: number): void {
		drafts.value = { ...drafts.value, [monitorKey(monitor)]: percent };
	}

	/** Al soltar (o con el teclado): se escribe, y sólo en ese monitor. */
	async function commit(monitor: MonitorBrightness, percent: number): Promise<void> {
		const key = monitorKey(monitor);
		preview(monitor, percent);
		try {
			await write(key, { monitor, percent });
		} catch (error) {
			console.error('[brightness] no se pudo cambiar el brillo:', error);
		} finally {
			// Si se volvió a arrastrar mientras tanto, el borrador nuevo se queda.
			if (drafts.value[key] === percent) {
				const { [key]: _done, ...rest } = drafts.value;
				drafts.value = rest;
			}
		}
	}

	return { monitors, primary, ddc, loaded, percentOf, preview, commit };
}
