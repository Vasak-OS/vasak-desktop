import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import type { TrayMenu } from '@/interfaces/tray';
import { hasActions, menuFocusTarget, trayMenuRows } from '@/tools/tray-menu';

const entry = (id: number, label: string, extra: Partial<TrayMenu> = {}): TrayMenu => ({
	id,
	label,
	enabled: true,
	visible: true,
	type: 'standard',
	...extra,
});

describe('trayMenuRows', () => {
	test('entradas, separadores y submenús en filas, en orden', () => {
		const rows = trayMenuRows([
			entry(1, 'Abrir'),
			entry(2, '', { type: 'separator' }),
			entry(3, 'Estado', {
				type: 'submenu',
				children: [entry(4, 'Disponible'), entry(5, 'Ausente')],
			}),
			entry(6, 'Salir'),
		]);
		expect(
			rows.map((row) =>
				row.kind === 'separator' ? 'sep' : `${row.kind}:${row.item.label}:${row.depth}`
			)
		).toEqual([
			'item:Abrir:0',
			'sep',
			'caption:Estado:0',
			'item:Disponible:1',
			'item:Ausente:1',
			'item:Salir:0',
		]);
	});

	test('lo invisible no se dibuja, ni lo que cuelga de ello', () => {
		const rows = trayMenuRows([
			entry(1, 'Abrir'),
			entry(2, 'Oculta', { visible: false }),
			entry(3, 'Grupo oculto', { visible: false, children: [entry(4, 'Hija')] }),
		]);
		expect(rows).toHaveLength(1);
	});

	test('un submenú vacío es un título, no una acción', () => {
		const rows = trayMenuRows([entry(1, 'Recientes', { type: 'submenu', children: [] })]);
		expect(rows.map((row) => row.kind)).toEqual(['caption']);
		expect(hasActions(rows)).toBe(false);
	});

	test('un submenú deshabilitado deshabilita a sus descendientes', () => {
		const rows = trayMenuRows([
			entry(1, 'Cuenta', {
				type: 'submenu',
				enabled: false,
				children: [
					entry(2, 'Cambiar'),
					entry(3, 'Más', { type: 'submenu', children: [entry(4, 'Honda')] }),
				],
			}),
			entry(5, 'Salir'),
		]);
		const enabled = Object.fromEntries(
			rows.flatMap((row) => (row.kind === 'separator' ? [] : [[row.item.label, row.item.enabled]]))
		);
		expect(enabled).toEqual({
			Cuenta: false,
			Cambiar: false,
			Más: false,
			Honda: false,
			Salir: true,
		});
	});

	test('sin entradas tocables no hay acciones, aunque haya separadores', () => {
		expect(hasActions(trayMenuRows([]))).toBe(false);
		expect(hasActions(trayMenuRows(undefined))).toBe(false);
		expect(hasActions(trayMenuRows([entry(1, '', { type: 'separator' })]))).toBe(false);
		expect(hasActions(trayMenuRows([entry(1, 'Salir')]))).toBe(true);
	});
});

describe('menuFocusTarget', () => {
	const enabled = [true, false, true, true];

	test('abajo y arriba saltan las deshabilitadas y dan la vuelta', () => {
		expect(menuFocusTarget(enabled, 0, 'ArrowDown')).toBe(2);
		expect(menuFocusTarget(enabled, 3, 'ArrowDown')).toBe(0);
		expect(menuFocusTarget(enabled, 2, 'ArrowUp')).toBe(0);
		expect(menuFocusTarget(enabled, 0, 'ArrowUp')).toBe(3);
	});

	test('sin foco todavía, abajo entra por la primera y arriba por la última', () => {
		expect(menuFocusTarget(enabled, -1, 'ArrowDown')).toBe(0);
		expect(menuFocusTarget(enabled, -1, 'ArrowUp')).toBe(3);
	});

	test('Inicio y Fin', () => {
		expect(menuFocusTarget(enabled, 2, 'Home')).toBe(0);
		expect(menuFocusTarget(enabled, 0, 'End')).toBe(3);
	});

	test('otra tecla, o nada habilitado, no mueve', () => {
		expect(menuFocusTarget(enabled, 0, 'Tab')).toBeNull();
		expect(menuFocusTarget([false, false], 0, 'ArrowDown')).toBeNull();
	});
});

/**
 * El alto del menú lo calcula Rust antes de abrir, con constantes que tienen
 * que ser las alturas de las clases de la vista. Si alguien cambia una sola de
 * las dos, el menú sale cortado o con aire, sin que nada falle.
 */
describe('las medidas del menú coinciden con las del backend', () => {
	const root = join(import.meta.dir, '..', '..');
	const rust = readFileSync(join(root, 'src-tauri/src/commands/tray.rs'), 'utf8');
	const view = readFileSync(join(root, 'src/views/applets/TrayPopupView.vue'), 'utf8');
	const popover = readFileSync(join(root, 'src/components/layouts/AppletPopover.vue'), 'utf8');
	const constant = (name: string) =>
		Number(rust.match(new RegExp(`const ${name}: f64 = ([0-9.]+);`))?.[1]);

	test.each([
		['TRAY_MENU_ITEM', 32, 'class="flex h-8 w-full'],
		['TRAY_MENU_CAPTION', 28, 'class="flex h-7 items-center'],
		['TRAY_MENU_SEPARATOR', 9, 'class="mx-2 my-1 h-px'],
	])('%s = %d px, y la vista usa esa altura', (name, px, cls) => {
		expect(constant(name)).toBe(px);
		expect(view).toContain(cls);
	});

	test('el contenedor compacto suma p-1 y el borde: 10 px', () => {
		expect(constant('TRAY_MENU_CHROME')).toBe(10);
		expect(popover).toContain("compact ? 'p-1' : 'p-4'");
		expect(view).toContain('<AppletPopover applet="tray" compact');
	});
});
