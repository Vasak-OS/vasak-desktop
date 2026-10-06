import { describe, expect, test } from 'bun:test';
import type { WidgetPlacement } from './catalog';
import {
	isPrimaryMonitor,
	monitorLabel,
	PRIMARY_MONITOR,
	readMonitorWidgets,
	withMonitorWidgets,
} from './storage';

const placement = (id: string): WidgetPlacement => ({
	id,
	type: 'clock',
	x: 1,
	y: 1,
	w: 2,
	h: 2,
});

describe('nombre de la salida', () => {
	test('sin parámetro —o vacío— es la principal', () => {
		expect(monitorLabel(undefined)).toBe(PRIMARY_MONITOR);
		expect(monitorLabel(null)).toBe(PRIMARY_MONITOR);
		expect(monitorLabel('')).toBe(PRIMARY_MONITOR);
	});

	test('un monitor secundario conserva su nombre', () => {
		expect(monitorLabel('desktop_1')).toBe('desktop_1');
	});

	test('sólo `desktop` es la principal', () => {
		expect(isPrimaryMonitor('desktop')).toBe(true);
		expect(isPrimaryMonitor('desktop_1')).toBe(false);
	});
});

describe('leer el layout de una salida', () => {
	test('la principal lee la clave heredada `desktop.widgets`', () => {
		const config = { desktop: { widgets: [placement('clock')] } };
		expect(readMonitorWidgets(config, 'desktop')).toEqual([placement('clock')]);
	});

	test('cada secundario lee su propia clave', () => {
		const config = {
			desktop: {
				widgets: [placement('principal')],
				widgetsByMonitor: {
					desktop_1: [placement('uno')],
					desktop_2: [placement('dos')],
				},
			},
		};

		expect(readMonitorWidgets(config, 'desktop_1')).toEqual([placement('uno')]);
		expect(readMonitorWidgets(config, 'desktop_2')).toEqual([placement('dos')]);
	});

	test('un secundario sin layout guardado da undefined, no el del principal', () => {
		const config = { desktop: { widgets: [placement('principal')] } };
		expect(readMonitorWidgets(config, 'desktop_1')).toBeUndefined();
	});

	test('un config vacío no revienta', () => {
		expect(readMonitorWidgets(undefined, 'desktop')).toBeUndefined();
		expect(readMonitorWidgets({}, 'desktop_1')).toBeUndefined();
	});
});

describe('guardar el layout de una salida', () => {
	test('la principal escribe en `desktop.widgets`, como siempre', () => {
		const next = withMonitorWidgets({ desktop: {} }, 'desktop', [placement('clock')]) as {
			desktop: { widgets: unknown };
		};
		expect(next.desktop.widgets).toEqual([placement('clock')]);
	});

	test('un secundario escribe en su clave sin tocar la del principal', () => {
		const config = { desktop: { widgets: [placement('principal')] } };
		const next = withMonitorWidgets(config, 'desktop_1', [placement('uno')]) as {
			desktop: { widgets: unknown; widgetsByMonitor: Record<string, unknown> };
		};

		expect(next.desktop.widgets).toEqual([placement('principal')]);
		expect(next.desktop.widgetsByMonitor.desktop_1).toEqual([placement('uno')]);
	});

	/**
	 * El caso que da sentido a todo: mover un widget en una pantalla no puede
	 * borrar el layout de la otra.
	 */
	test('guardar un secundario conserva el layout de los otros secundarios', () => {
		const config = {
			desktop: { widgetsByMonitor: { desktop_1: [placement('uno')] } },
		};
		const next = withMonitorWidgets(config, 'desktop_2', [placement('dos')]) as {
			desktop: { widgetsByMonitor: Record<string, unknown> };
		};

		expect(next.desktop.widgetsByMonitor.desktop_1).toEqual([placement('uno')]);
		expect(next.desktop.widgetsByMonitor.desktop_2).toEqual([placement('dos')]);
	});

	test('conserva el resto de la configuración (estilo, fondo, otras claves)', () => {
		const config = {
			style: { darkmode: true },
			desktop: { wallpaper: ['/un/fondo.jpg'], showfiles: true },
		};
		const next = withMonitorWidgets(config, 'desktop', [placement('clock')]) as {
			style: unknown;
			desktop: { wallpaper: unknown; showfiles: unknown };
		};

		expect(next.style).toEqual({ darkmode: true });
		expect(next.desktop.wallpaper).toEqual(['/un/fondo.jpg']);
		expect(next.desktop.showfiles).toBe(true);
	});

	test('un config vacío no revienta al guardar', () => {
		const next = withMonitorWidgets(undefined, 'desktop_1', [placement('uno')]) as {
			desktop: { widgetsByMonitor: Record<string, unknown> };
		};
		expect(next.desktop.widgetsByMonitor.desktop_1).toEqual([placement('uno')]);
	});
});
