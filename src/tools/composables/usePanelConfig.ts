import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { computed } from 'vue';
import { esVertical, posicionDelPanel } from '@/tools/posicion-del-panel';

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

	const seccion = computed(() => (configStore as any).config?.panel ?? {});
	const posicion = computed(() => posicionDelPanel((configStore as any).config));

	return {
		showWeather: computed(() => seccion.value.weather !== false),
		showMusic: computed(() => seccion.value.music !== false),
		showTransfer: computed(() => seccion.value.transfer !== false),
		showTray: computed(() => seccion.value.tray !== false),
		showPrivacy: computed(() => seccion.value.privacy !== false),
		posicion,
		/** `true` cuando el panel es una columna, que es lo que más se pregunta. */
		vertical: computed(() => esVertical(posicion.value)),
	};
}
