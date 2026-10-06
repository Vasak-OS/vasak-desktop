/**
 * El escritorio del monitor secundario también ofrece el clic derecho y la
 * edición de widgets (vasak-desktop#164).
 *
 * Antes `WidgetLayer` —donde viven el menú del clic derecho (`onContextMenu`,
 * `showContextMenu`) y el modo edición— se dibujaba sólo con
 * `v-if="!isSecondaryMonitor"`, así que el secundario se quedaba sin menú ni
 * edición. Ahora se dibuja en todos los monitores, cada uno con su propio layout
 * (`monitorId`), para no duplicar los widgets del principal ni pisarlos.
 *
 * Se mira el texto de los componentes, al estilo de `widgets-music-weather`: lo
 * que importa es cómo quedan cableados, no montarlos.
 */
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = join(import.meta.dir, '..');
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

const DESKTOP_VIEW = read('src/views/DesktopView.vue');
const WIDGET_LAYER = read('src/components/widgets/WidgetLayer.vue');

describe('la capa de widgets está en todos los monitores', () => {
	test('DesktopView ya no esconde WidgetLayer en el secundario', () => {
		expect(DESKTOP_VIEW).not.toContain('v-if="!isSecondaryMonitor"');
	});

	test('DesktopView dibuja WidgetLayer con la salida de cada monitor', () => {
		expect(DESKTOP_VIEW).toMatch(/<WidgetLayer[\s\S]*?:monitor-id="monitorId"/);
	});

	test('la salida sale del parámetro ?monitor= de la ventana', () => {
		expect(DESKTOP_VIEW).toContain("route.query.monitor as string) || 'desktop'");
	});

	/**
	 * El menú y la edición siguen viviendo en WidgetLayer: si esto se rompiera,
	 * habilitar la capa en el secundario no serviría de nada.
	 */
	test('el clic derecho y la edición siguen en WidgetLayer', () => {
		expect(WIDGET_LAYER).toContain('showContextMenu');
		expect(WIDGET_LAYER).toContain("window.addEventListener('contextmenu', onContextMenu)");
		expect(WIDGET_LAYER).toContain('editing.value = true');
	});
});

describe('cada monitor guarda su layout por separado', () => {
	test('WidgetLayer recibe la salida y la usa para leer y guardar', () => {
		expect(WIDGET_LAYER).toMatch(/defineProps<\{\s*config: any;\s*monitorId\?: string\s*\}>/);
		expect(WIDGET_LAYER).toContain('readMonitorWidgets(props.config, monitor.value)');
		expect(WIDGET_LAYER).toContain('withMonitorWidgets(props.config, monitor.value, placements.value)');
	});

	/**
	 * El secundario no arranca con la disposición de siempre (`allowDefault`
	 * falso), para no duplicar el reloj y la música del principal en cada
	 * pantalla.
	 */
	test('el default de fábrica es sólo del principal', () => {
		expect(WIDGET_LAYER).toContain('isPrimaryMonitor(monitor.value)');
		expect(WIDGET_LAYER).toMatch(/resolveLayout\([\s\S]*?allowDefault\.value/);
	});
});
