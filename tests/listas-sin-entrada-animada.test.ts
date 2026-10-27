/**
 * §5.5 de once-ui: «La lista no anima sus filas al entrar: una sola animación,
 * la del panel». Las listas del escritorio (íconos del tray, notificaciones del
 * centro de control y de cada grupo) conservan `move` (reordenar) y `leave`
 * (descartar), pero ninguna fila entra animada: las notificaciones llegan async
 * al abrir y entraban en cascada.
 */
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { Glob } from 'bun';
import { ROOT } from './support/mount-sfc';

const LISTS = [
	'src/components/areas/panel/TrayBarArea.vue',
	'src/components/cards/NotificationGroupCard.vue',
	'src/components/areas/control-center/NotificationArea.vue',
];

/** Etiquetas de apertura de cada `<TransitionGroup ...>` de la plantilla (`>` dentro de comillas no corta). */
const groupTags = (file: string): string[] =>
	[...readFileSync(join(ROOT, file), 'utf8').matchAll(/<TransitionGroup\b(?:"[^"]*"|'[^']*'|[^>"'])*>/g)].map((m) => m[0]);

describe('las listas no animan sus filas al entrar (§5.5)', () => {
	for (const file of LISTS) {
		test(`${file} no declara clases de entrada y conserva move y leave`, () => {
			const tags = groupTags(file);
			expect(tags).toHaveLength(1);
			expect(tags[0]).not.toMatch(/\benter-(active|from|to)-class\b/);
			expect(tags[0]).toMatch(/\bmove-class\b/);
			expect(tags[0]).toMatch(/\bleave-active-class\b/);
			expect(tags[0]).toMatch(/\bleave-to-class\b/);
		});
	}

	test('ningún TransitionGroup del escritorio anima la entrada de sus filas', () => {
		const offenders: string[] = [];
		for (const file of new Glob('src/**/*.vue').scanSync({ cwd: ROOT })) {
			for (const tag of groupTags(file)) {
				if (/\benter-(active|from|to)-class\b/.test(tag)) offenders.push(file);
			}
		}
		expect(offenders).toEqual([]);
	});
});
