import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import type { Store } from 'pinia';
import { computed } from 'vue';

/**
 * Qué muestra el panel.
 *
 * Todo arranca encendido: quien nunca abrió la configuración tiene que ver el
 * panel completo, y la sección de `panel` en el archivo no existe hasta que
 * alguien apaga algo. Por eso la pregunta es siempre `!== false` y no `=== true`:
 * la ausencia de la clave significa «mostralo», no «escondelo».
 */
export function usePanelConfig() {
	const configStore = useConfigStore() as Store<'config', { config: any }>;

	const seccion = computed(() => (configStore as any).config?.panel ?? {});

	return {
		showWeather: computed(() => seccion.value.weather !== false),
		showMusic: computed(() => seccion.value.music !== false),
		showTransfer: computed(() => seccion.value.transfer !== false),
		showTray: computed(() => seccion.value.tray !== false),
	};
}
