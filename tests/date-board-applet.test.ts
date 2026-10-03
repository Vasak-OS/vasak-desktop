/**
 * El tablero de fecha que cuelga del reloj del panel (vasak-desktop#130).
 *
 * Lo que dibujan el mes, la lista, el arco y los anillos está probado en
 * vue-libvasak, montado; las cuentas, en `src/tools/date-board.test.ts`. Lo que
 * se fija acá es cómo se arma el tablero en el escritorio, que es lo que se
 * separa sin avisar: un reloj que deja de abrirlo, un applet que abre sin
 * permisos porque su ventana no está en la capability, una segunda fuente de
 * eventos al lado del almacén local, un segundo pedido de clima al lado de
 * `useWeather`, o un texto que falta en un idioma y se ve como clave cruda.
 *
 * Se mira el texto y no se monta, como `music-applet.test.ts`.
 */
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = join(import.meta.dir, '..');
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

const VIEW = read('src/views/applets/DateBoardAppletView.vue');
const CLOCK = read('src/components/widgets/PanelClockWidget.vue');
const ROUTES = read('src/routes/index.ts');
const APPLETS = read('src-tauri/src/windows_apps/anchored_applet.rs');
const WEATHER = read('src/tools/forecast-url.ts');
const CAPABILITY = JSON.parse(read('src-tauri/capabilities/default.json')) as { windows: string[] };
const ES = Bun.YAML.parse(read('src-tauri/locales/es.yml')) as Record<string, any>;
const EN = Bun.YAML.parse(read('src-tauri/locales/en.yml')) as Record<string, any>;

describe('el tablero de fecha', () => {
	test('lo abre el reloj del panel, colgado de él, y el reloj se realza mientras está abierto', () => {
		expect(CLOCK).toContain("toggleApplet('date', opener.value)");
		expect(CLOCK).toContain('ref="opener"');
		expect(CLOCK).toContain("useOpenApplet('date')");
		expect(CLOCK).toMatch(/<button[\s\S]*?type="button"/);
		expect(CLOCK).toContain(':aria-expanded="isOpen"');
	});

	test('es un applet anclado: fila en APPLETS, ruta y ventana en la capability', () => {
		expect(VIEW).toContain('<AppletPopover applet="date"');
		expect(APPLETS).toContain('AppletSpec { id: "date", route: "date", size: (960.0, 540.0) }');
		expect(ROUTES).toContain("path: 'date'");
		expect(ROUTES).toContain("import('@/views/applets/DateBoardAppletView.vue')");
		expect(CAPABILITY.windows).toContain('applet_date');
	});

	test('dibuja las piezas de la librería, no unas propias', () => {
		for (const piece of ['MonthCalendar', 'EventList', 'HourlyForecast', 'ProgressRing', 'ClockDisplay', 'EmptyState']) {
			expect(VIEW).toMatch(new RegExp(`import \\{[^}]*\\b${piece}\\b[^}]*\\} from '@vasakgroup/vue-libvasak'`));
			expect(VIEW).toContain(`<${piece}`);
		}
		// Las cuentas de fechas también son las de la librería.
		expect(VIEW).toContain('markedDates(');
		expect(VIEW).toContain('entriesOn(');
	});

	test('los eventos salen sólo del almacén local de vasak-accounts', () => {
		expect(VIEW).toContain("from '@/services/calendar.service'");
		expect(VIEW).toContain('calendarOccurrences(');
		expect(VIEW).not.toMatch(/fetch\(|caldav|CalDAV|invoke\s*[<(]/);
	});

	test('sin servicio o sin permiso el bloque lo dice con un vacío, nunca roto', () => {
		expect(VIEW).toContain("reply?.state === 'unavailable'");
		expect(VIEW).toContain("reply?.state === 'denied'");
		expect(VIEW).toContain("reply?.state === 'failed'");
		expect(VIEW).toContain("t('views.dateBoard.unavailable')");
		expect(VIEW).toContain("t('views.dateBoard.denied')");
		// Y un día sin eventos tampoco queda en blanco.
		expect(VIEW).toContain(":empty-label=\"t('views.dateBoard.noEvents')\"");
	});

	test('el clima es el de useWeather, el mismo del widget y del menú', () => {
		expect(VIEW).toContain('useWeather()');
		expect(VIEW).not.toContain('open-meteo');
		// Y el pedido compartido trae lo que el tablero necesita.
		for (const field of ['precipitation_probability', 'relative_humidity_2m', 'apparent_temperature', 'wind_speed_10m']) {
			expect(WEATHER).toContain(`'${field}'`);
		}
		expect(WEATHER).toContain('hourly=');
	});

	test('el reloj grande va a 24 horas, como el del panel, con los segundos chicos', () => {
		expect(VIEW).toContain('<ClockDisplay size="lg" seconds small-seconds :hour12="false" />');
	});

	test('en una ventana angosta las columnas se apilan por contenedor, no por la pantalla', () => {
		expect(VIEW).toContain('@container');
		expect(VIEW).toContain('@3xl:grid-cols-');
		expect(VIEW).not.toMatch(/(?<![\w@-])(?:sm|md|lg|xl):[a-z]/);
	});
});

describe('los textos del tablero', () => {
	const keysIn = (text: string) =>
		[...new Set([...text.matchAll(/t\('(views\.dateBoard\.[\w.]+|components\.PanelClock\.[\w.]+)'\)/g)].map((m) => m[1] as string))];
	const lookup = (catalog: Record<string, any>, key: string) =>
		key.split('.').reduce<any>((node, part) => (node && typeof node === 'object' ? node[part] : undefined), catalog);

	test('cada texto que pide el tablero está en los dos idiomas', () => {
		const keys = [...keysIn(VIEW), ...keysIn(CLOCK)];
		expect(keys.length).toBeGreaterThan(20);
		for (const key of keys) {
			expect(typeof lookup(ES, key), `falta ${key} en es.yml`).toBe('string');
			expect(typeof lookup(EN, key), `falta ${key} en en.yml`).toBe('string');
		}
	});

	test('cada condición del clima tiene su nombre en los dos idiomas', () => {
		const codes = Object.keys(JSON.parse(read('src/data/weatherCodes.json')));
		expect(codes.length).toBe(28);
		for (const code of codes) {
			expect(typeof ES.views.dateBoard.conditions[code], `falta la condición ${code} en es.yml`).toBe('string');
			expect(typeof EN.views.dateBoard.conditions[code], `falta la condición ${code} en en.yml`).toBe('string');
		}
	});
});
