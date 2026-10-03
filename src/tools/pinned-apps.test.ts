import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { PINNED_APPS } from './pinned-apps';

const ROOT = join(import.meta.dir, '../..');

describe('los accesos fijos del panel', () => {
	test('son Configuración y Archivos, los de siempre', () => {
		expect(PINNED_APPS.map((app) => app.id)).toEqual(['settings', 'files']);
		expect(PINNED_APPS.map((app) => app.icon)).toEqual([
			'preferences-system',
			'system-file-manager',
		]);
	});

	test('cada uno se puede lanzar: está en shell:allow-spawn', () => {
		const capability = readFileSync(join(ROOT, 'src-tauri/capabilities/default.json'), 'utf8');
		for (const app of PINNED_APPS) expect(capability).toContain(`"name": "${app.command}"`);
	});

	test('cada nombre está en los dos catálogos', () => {
		for (const lang of ['es', 'en']) {
			const catalog = Bun.YAML.parse(
				readFileSync(join(ROOT, `src-tauri/locales/${lang}.yml`), 'utf8')
			) as any;
			for (const app of PINNED_APPS) {
				const value = app.label.split('.').reduce((node: any, key) => node?.[key], catalog);
				expect(typeof value, `${lang}: ${app.label}`).toBe('string');
			}
		}
	});
});
