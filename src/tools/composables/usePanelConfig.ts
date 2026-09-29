import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { computed } from 'vue';
import { isVertical, panelPosition } from '@/tools/panel-position';

/**
 * Qué muestra el panel y de qué lado va.
 *
 * Todo arranca encendido: quien nunca abrió la configuración tiene que ver el
 * panel completo, y la sección de `panel` en el archivo no existe hasta que
 * alguien apaga algo. Por eso la pregunta es siempre `!== false` y no `=== true`:
 * la ausencia de la clave significa «mostralo», no «escondelo».
 *
 * La posición sale de la misma sección y se lee igual de tolerante: sin clave,
 * arriba. Es reactiva como el resto porque `App.vue` recarga la configuración
 * con cada `config-changed`, así que mover el panel en Configuración reacomoda
 * lo de adentro sin reiniciar nada — el backend, mientras tanto, reancla la
 * superficie.
 */
export function usePanelConfig() {
	const configStore = useConfigStore();

	const section = computed(() => (configStore as any).config?.panel ?? {});
	const position = computed(() => panelPosition((configStore as any).config));

	return {
		showWeather: computed(() => section.value.weather !== false),
		showMusic: computed(() => section.value.music !== false),
		showTransfer: computed(() => section.value.transfer !== false),
		showTray: computed(() => section.value.tray !== false),
		showPrivacy: computed(() => section.value.privacy !== false),
		position,
		/** `true` cuando el panel es una columna, que es lo que más se pregunta. */
		vertical: computed(() => isVertical(position.value)),
	};
}
