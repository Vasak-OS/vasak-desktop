import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { computed } from 'vue';
import {
	hasSurface,
	panelAnimation,
	panelAnimationClass,
	panelBarClasses,
	panelLayout,
	panelStyle,
	panelSurfaceClass,
} from '@/tools/panel-appearance';
import { readPanelAutohide } from '@/tools/panel-autohide';
import { isVertical, panelPosition } from '@/tools/panel-position';
import { resolvePinnedApps } from '@/tools/pinned-apps';

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

	const config = computed(() => (configStore as any).config);
	const section = computed(() => config.value?.panel ?? {});
	const position = computed(() => panelPosition(config.value));

	/**
	 * El aspecto de la barra: su tipo, su densidad, su animación y su tamaño.
	 * Todo sale de la misma sección `panel` y se lee tolerante (`panel-appearance.ts`).
	 * Es reactivo como el resto: `App.vue` recarga con cada `config-changed`, así
	 * que cambiar el aspecto en Configuración reacomoda la barra sin reiniciar.
	 */
	const style = computed(() => panelStyle(config.value));
	const layout = computed(() => panelLayout(config.value));
	const animation = computed(() => panelAnimation(config.value));

	return {
		showWeather: computed(() => section.value.weather !== false),
		showMusic: computed(() => section.value.music !== false),
		showTransfer: computed(() => section.value.transfer !== false),
		showTray: computed(() => section.value.tray !== false),
		showPrivacy: computed(() => section.value.privacy !== false),
		showKeyboard: computed(() => section.value.keyboard !== false),
		/** `true` salvo que `panel.workspaces` esté en `false`: la píldora de espacios de trabajo. */
		showWorkspaces: computed(() => section.value.workspaces !== false),
		/** `true` salvo que `panel.windows` esté en `false`: la píldora de ventanas abiertas. */
		showWindows: computed(() => section.value.windows !== false),
		/** `true` salvo que `panel.networkName` esté en `false`: el nombre de la red en su píldora. */
		showNetworkName: computed(() => section.value.networkName !== false),
		/** `true` salvo que `panel.volumeNumber` esté en `false`: el número del volumen en su píldora. */
		showVolumeNumber: computed(() => section.value.volumeNumber !== false),
		/** `true` salvo que `panel.batteryNumber` esté en `false`: el número de la batería en su píldora. */
		showBatteryNumber: computed(() => section.value.batteryNumber !== false),
		/**
		 * Las aplicaciones ancladas: la lista de `panel.pinned`, o los dos por
		 * defecto cuando está ausente (ver `resolvePinnedApps`).
		 */
		pinnedApps: computed(() => resolvePinnedApps(section.value)),
		position,
		/** `true` cuando el panel es una columna, que es lo que más se pregunta. */
		vertical: computed(() => isVertical(position.value)),
		style,
		layout,
		/** Las clases de la `<nav>` según el tipo y la densidad. */
		barClasses: computed(() => panelBarClasses(position.value, style.value, layout.value)),
		/** Las clases de la superficie, o `''` en píldoras (que no dibuja ninguna). */
		surfaceClass: computed(() => panelSurfaceClass(style.value, position.value)),
		/** `true` cuando hay una superficie continua: la región de entrada es la barra entera. */
		hasSurface: computed(() => hasSurface(style.value)),
		/** La clase que enciende la animación elegida, o `''` si está apagada. */
		animationClass: computed(() => panelAnimationClass(animation.value)),
		/** `true` cuando el panel se esconde solo y se revela al rozar el borde. */
		autohide: computed(() => readPanelAutohide(config.value)),
	};
}
