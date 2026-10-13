import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { DEFAULT_PINNED, type PinnedApp, pinnedLabel, resolvePinnedApps } from './pinned-apps';

const ROOT = join(import.meta.dir, '../..');

describe('los anclados por defecto', () => {
	test('son Configuración y Archivos, los de siempre', () => {
		expect(DEFAULT_PINNED.map((app) => app.icon)).toEqual([
			'preferences-system',
			'system-file-manager',
		]);
	});

	test('su `id` es la ruta del `.desktop`: lo que recibe `open_app`', () => {
		// Y no un id suelto ni la línea `Exec`: `open_app` abre el archivo en esa
		// ruta y lee su `Exec=`. Por eso anclar es guardar la ruta del `.desktop`.
		for (const app of DEFAULT_PINNED) {
			expect(app.id.startsWith('/')).toBe(true);
			expect(app.id.endsWith('.desktop')).toBe(true);
		}
	});

	test('su nombre es una clave del catálogo, y está en los dos idiomas', () => {
		for (const lang of ['es', 'en']) {
			const catalog = Bun.YAML.parse(
				readFileSync(join(ROOT, `src-tauri/locales/${lang}.yml`), 'utf8')
			) as any;
			for (const app of DEFAULT_PINNED) {
				expect(app.labelKey, app.name).toBe(true);
				const value = app.name.split('.').reduce((node: any, key) => node?.[key], catalog);
				expect(typeof value, `${lang}: ${app.name}`).toBe('string');
			}
		}
	});
});

describe('qué lista de anclados corresponde a una configuración', () => {
	const sample: PinnedApp = {
		id: '/usr/share/applications/vasak-terminal.desktop',
		name: 'Terminal',
		icon: 'utilities-terminal',
		exec: 'vasak-terminal',
	};

	test('sin la clave `pinned` van los dos por defecto', () => {
		expect(resolvePinnedApps(undefined)).toBe(DEFAULT_PINNED);
		expect(resolvePinnedApps({})).toBe(DEFAULT_PINNED);
		expect(resolvePinnedApps(null)).toBe(DEFAULT_PINNED);
		// Algo que no es un arreglo tampoco cuenta como lista.
		expect(resolvePinnedApps({ pinned: 'nada' })).toBe(DEFAULT_PINNED);
	});

	test('una lista vacía es una decisión: ningún anclado, no los por defecto', () => {
		// Ausente y «vacía» tienen que ser distinguibles, o apagar todos los
		// anclados sería imposible de expresar.
		expect(resolvePinnedApps({ pinned: [] })).toEqual([]);
	});

	test('una lista presente se usa tal cual: pisa, no suma a los por defecto', () => {
		const result = resolvePinnedApps({ pinned: [sample] });
		expect(result).toEqual([sample]);
	});
});

describe('el nombre visible de un anclado', () => {
	const t = (key: string) => `traducido:${key}`;

	test('los por defecto se traducen (son claves del catálogo)', () => {
		const [settings] = DEFAULT_PINNED;
		expect(settings).toBeDefined();
		expect(pinnedLabel(settings as PinnedApp, t)).toBe('traducido:views.panel.settingsAlt');
	});

	test('los de la configuración van literales: su nombre lo da el sistema', () => {
		const app: PinnedApp = {
			id: '/x.desktop',
			name: 'Mi App',
			icon: 'x',
			exec: 'mi-app',
		};
		expect(pinnedLabel(app, t)).toBe('Mi App');
	});
});
