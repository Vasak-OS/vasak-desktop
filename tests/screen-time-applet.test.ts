/**
 * El tablero de tiempo de pantalla (vasak-desktop#150), como lo arma el
 * escritorio.
 *
 * Lo que dibujan las barras, el mapa de calor y la fila con barra está probado
 * montado en vue-libvasak; las cuentas, en `src/tools/screen-time.test.ts`; la
 * contabilidad y el archivo, en Rust (`src-tauri/src/screen_time/`). Lo que se
 * fija acá es cómo se juntan, que es lo que se separa sin avisar: un applet que
 * abre sin permisos, una pieza dibujada a mano al lado de la de la librería, un
 * tablero vacío que no dice nada, o un registro que no se puede apagar.
 *
 * Se mira el texto y no se monta porque este repositorio todavía no tiene con
 * qué montar, igual que `music-applet.test.ts`.
 */
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = join(import.meta.dir, '..');
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

const VIEW = read('src/views/applets/ScreenTimeAppletView.vue');
const CONTROL = read('src/components/controls/ScreenTimeControl.vue');
const CENTER = read('src/views/ControlCenterView.vue');
const ROUTES = read('src/routes/index.ts');
const SERVICE = read('src/services/screen-time.service.ts');
const APPLETS = read('src-tauri/src/windows_apps/anchored_applet.rs');
const LIB = read('src-tauri/src/lib.rs');
const DBUS = read('src-tauri/src/dbus_service.rs');
const CAPABILITY = JSON.parse(read('src-tauri/capabilities/default.json')) as { windows: string[] };
const ES = Bun.YAML.parse(read('src-tauri/locales/es.yml')) as Record<string, any>;
const EN = Bun.YAML.parse(read('src-tauri/locales/en.yml')) as Record<string, any>;
const template = VIEW.slice(VIEW.indexOf('<template>'), VIEW.lastIndexOf('</template>'));

describe('el tablero de tiempo de pantalla', () => {
	test('lo abre un botón del centro de control, como applet anclado', () => {
		expect(CENTER).toContain('<ScreenTimeControl />');
		expect(CONTROL).toContain("toggleApplet('screen-time')");
		// Primero se va el centro de control: el tablero ocupa el centro.
		expect(CONTROL.indexOf('hideControlCenter()')).toBeLessThan(CONTROL.indexOf("toggleApplet('screen-time')"));
		expect(CONTROL).toContain('<ToggleControl');
	});

	test('tiene su fila en APPLETS, su ruta y su ventana en la capability', () => {
		expect(APPLETS).toContain('AppletSpec { id: "screen-time", route: "screen-time", size: (580.0, 590.0) }');
		expect(ROUTES).toContain("path: 'screen-time'");
		expect(ROUTES).toContain("import('@/views/applets/ScreenTimeAppletView.vue')");
		expect(CAPABILITY.windows).toContain('applet_screen-time');
		expect(template).toContain('<AppletPopover applet="screen-time"');
	});

	test('los comandos están registrados y el borrado también va por D-Bus', () => {
		expect(LIB).toContain('screen_time::screen_time_range');
		expect(LIB).toContain('screen_time::screen_time_clear');
		expect(LIB).toContain('screen_time::start(app.handle())');
		expect(DBUS).toContain('"ClearScreenTime"');
		expect(SERVICE).toContain("invoke<ScreenTimeRange>('screen_time_range', { from, to })");
		expect(SERVICE).toContain("invoke<void>('screen_time_clear')");
	});

	test('las piezas son de la librería: nada dibujado a mano', () => {
		for (const piece of ['<BarChart', '<CalendarHeatmap', '<ListRow', '<StatTile', '<SwitchToggle', '<ActionButton']) {
			expect(template).toContain(piece);
		}
		expect(template).not.toMatch(/<svg|<button|<input/);
		// La fila de cada aplicación: icono del tema, barra proporcional, el
		// tiempo escrito y el realce al pasar.
		expect(template).toContain(':bar="app.share"');
		expect(template).toContain(':icon="app.icon"');
		expect(template).toContain(':meta="duration(app.ms)"');
		expect(template).toContain('hoverable');
	});

	test('«‹ Hoy ›»: los dos botones de día, con nombre, y no se va al futuro', () => {
		expect(template).toContain('icon="go-previous"');
		expect(template).toContain('icon="go-next"');
		expect(template).toContain(':disabled="!canGoForward"');
		expect(template).toContain(':disabled="!canGoBack"');
		expect(VIEW).toContain("t('views.screenTimeApplet.today')");
	});

	test('el tablero vacío dice por qué, y distingue apagado de sin datos', () => {
		expect(template).toContain('data-screen-time-empty');
		expect(template).toContain('data-screen-time-disabled');
		expect(template).toContain("t('views.screenTimeApplet.empty')");
		expect(template).toContain("t('views.screenTimeApplet.disabled')");
		// Y la lista no se dibuja vacía al lado del aviso.
		expect(template).toMatch(/v-for="app in apps"\s+v-else/);
	});

	test('el registro se apaga y el historial se borra desde el tablero, con confirmación', () => {
		expect(template).toContain('<SwitchToggle');
		expect(template).toContain('@update:model-value="toggle"');
		expect(SERVICE).toContain('screen_time: { ...section, enabled }');
		// Borrar pide un segundo paso.
		expect(template).toContain('@click="confirming = true"');
		expect(template).toContain('@click="clearAll"');
	});

	test('una respuesta vieja no pisa el día que se está mirando', () => {
		expect(VIEW).toContain('const ticket = latest;');
		expect(VIEW.match(/if \(ticket !== latest\) return;/g)).toHaveLength(2);
		expect(VIEW.indexOf('if (ticket !== latest) return;')).toBeLessThan(VIEW.indexOf('range.value = response;'));
	});

	test('una columna por vez en angosto, por contenedor y no por la pantalla', () => {
		expect(template).toContain('@container');
		expect(template).toContain('@[30rem]:grid-cols-3');
		expect(template).toContain('@[30rem]:flex-row');
		expect(template).toContain('overflow-y-auto');
		expect(VIEW).not.toMatch(/(?<![\w@-])(?:sm|md|lg|xl):/);
	});

	test('todo texto está en los dos idiomas', () => {
		const keys = [...VIEW.matchAll(/t\('views\.screenTimeApplet\.([\w.]+)'\)/g)].map((match) => match[1] as string);
		expect(keys.length).toBeGreaterThan(15);
		for (const key of keys) {
			const path = key.split('.');
			const es = path.reduce((node, part) => node?.[part], ES.views.screenTimeApplet);
			const en = path.reduce((node, part) => node?.[part], EN.views.screenTimeApplet);
			expect(typeof es, `es: ${key}`).toBe('string');
			expect(typeof en, `en: ${key}`).toBe('string');
		}
		expect(ES.components.ScreenTimeControl.open).toBe('Tiempo de pantalla');
		expect(EN.components.ScreenTimeControl.open).toBe('Screen time');
	});
});
