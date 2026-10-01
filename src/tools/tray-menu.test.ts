import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import type { TrayMenu } from '@/interfaces/tray';
import {
	DISPOSITION_STYLE,
	entryCheck,
	entryRole,
	hasActions,
	menuFocusTarget,
	shortcutChords,
	shortcutLabel,
	toggleIconName,
	trayMenuRows,
} from '@/tools/tray-menu';

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

	// Las filas son los componentes del menú de la librería: las alturas salen
	// de sus clases, que se leen del paquete instalado y no de una copia.
	const library = readFileSync(
		join(root, 'node_modules/@vasakgroup/vue-libvasak/dist/vue-libvasak.es.js'),
		'utf8'
	);

	test.each([
		// min-h-8, y la etiqueta en una línea (`truncate`): 32 px.
		['TRAY_MENU_ITEM', 32, 'flex min-h-8 min-w-0 items-center', '<DropdownMenuItem'],
		// pt-2 + pb-1 + la línea de text-label-xs (1rem): 28 px.
		['TRAY_MENU_CAPTION', 28, 'px-3 pt-2 pb-1 font-semibold text-label-xs', '<DropdownMenuLabel'],
		// my-1 + h-px: 9 px.
		['TRAY_MENU_SEPARATOR', 9, '-mx-1 my-1 h-px', '<DropdownMenuSeparator'],
	])('%s = %d px, y la vista usa el componente que mide eso', (name, px, cls, tag) => {
		expect(constant(name)).toBe(px);
		expect(library).toContain(cls);
		expect(view).toContain(tag);
	});

	test('la etiqueta de cada fila no se parte en dos líneas', () => {
		// Una etiqueta partida haría la fila más alta que lo que midió el backend.
		expect(view).toContain('<span class="min-w-0 flex-1 truncate">{{ row.item.label }}</span>');
		expect(view).toMatch(/<DropdownMenuLabel[^>]*class="truncate"/);
	});

	test('el contenedor compacto suma p-1 y el borde: 10 px', () => {
		expect(constant('TRAY_MENU_CHROME')).toBe(10);
		expect(popover).toContain("compact ? 'p-1' : 'p-4'");
		expect(view).toContain('<AppletPopover applet="tray" compact');
	});
});

describe('lo que trae cada entrada', () => {
	test('el atajo se escribe como en un menú', () => {
		expect(shortcutLabel([['Control', 'q']])).toBe('Ctrl+Q');
		expect(
			shortcutLabel([
				['Control', 'Q'],
				['Alt', 'X'],
			])
		).toBe('Ctrl+Q, Alt+X');
		expect(shortcutLabel([['Super', 'Shift', 'Delete']])).toBe('Super+Shift+Delete');
		expect(shortcutLabel(undefined)).toBeUndefined();
		expect(shortcutLabel([[]])).toBeUndefined();
	});

	test('casilla, radio e indeterminado tienen su rol y su estado', () => {
		const e = (toggle?: TrayMenu['toggle']) => entry(1, 'x', { toggle });
		expect(entryRole(e())).toEqual({ role: 'menuitem' });
		expect(entryRole(e({ kind: 'checkmark', state: 'on' }))).toEqual({
			role: 'menuitemcheckbox',
			checked: 'true',
		});
		expect(entryRole(e({ kind: 'checkmark', state: 'indeterminate' }))).toEqual({
			role: 'menuitemcheckbox',
			checked: 'mixed',
		});
		// Radio no admite `mixed` en ARIA.
		expect(entryRole(e({ kind: 'radio', state: 'indeterminate' }))).toEqual({
			role: 'menuitemradio',
			checked: 'false',
		});
	});

	test('el indicador sale del tema, distinto para cada estado', () => {
		expect(toggleIconName(undefined)).toBeUndefined();
		expect(toggleIconName({ kind: 'checkmark', state: 'on' })).toBe('checkbox-checked-symbolic');
		expect(toggleIconName({ kind: 'checkmark', state: 'off' })).toBe('checkbox-symbolic');
		expect(toggleIconName({ kind: 'checkmark', state: 'indeterminate' })).toBe(
			'checkbox-mixed-symbolic'
		);
		expect(toggleIconName({ kind: 'radio', state: 'on' })).toBe('radio-checked-symbolic');
		expect(toggleIconName({ kind: 'radio', state: 'indeterminate' })).toBe('radio-mixed-symbolic');
	});

	test('cada disposición tiene su tono y su icono', () => {
		expect(DISPOSITION_STYLE.informative).toEqual({
			tone: 'info',
			icon: 'dialog-information-symbolic',
		});
		expect(DISPOSITION_STYLE.warning.tone).toBe('warning');
		expect(DISPOSITION_STYLE.alert).toEqual({ tone: 'error', icon: 'dialog-error-symbolic' });
	});

	test('la vista dibuja cada cosa sólo si viene', () => {
		const view = readFileSync(
			join(import.meta.dir, '..', 'views', 'applets', 'TrayPopupView.vue'),
			'utf8'
		);
		// La columna del icono sólo se ocupa si hay algo que poner, y la
		// reserva de las demás va por `inset`, no por un hueco vacío.
		expect(view).toContain('ownsLeadingColumn(item) ? 1 : 0');
		expect(view).toContain('v-if="row.item.disposition"');
		expect(view).toContain('v-if="shortcutLabel(row.item.shortcut)" #shortcut');
	});

	test('la vista usa la marca, la sangría y las teclas de la librería', () => {
		const view = readFileSync(
			join(import.meta.dir, '..', 'views', 'applets', 'TrayPopupView.vue'),
			'utf8'
		);
		expect(view).toContain(':checked="entryCheck(row.item).checked"');
		expect(view).toContain(':inset="entryInset(row.item, row.depth)"');
		expect(view).toContain('<Kbd :keys="chord"');
		// Ni el `role` suelto para todas ni la sangría en línea de la 2.0.
		expect(view).not.toContain('entryAttrs');
		expect(view).not.toMatch(/<DropdownMenuItem[^>]*paddingLeft/);
	});
});

describe('la marca y el atajo, para la librería', () => {
	const e = (toggle?: TrayMenu['toggle']) => entry(1, 'x', { toggle });

	test('prendido y apagado son `checked`; sin casilla, `null`', () => {
		expect(entryCheck(e())).toEqual({ checked: null, toggle: 'checkbox' });
		expect(entryCheck(e({ kind: 'checkmark', state: 'on' }))).toEqual({ checked: true, toggle: 'checkbox' });
		expect(entryCheck(e({ kind: 'checkmark', state: 'off' }))).toEqual({ checked: false, toggle: 'checkbox' });
		expect(entryCheck(e({ kind: 'radio', state: 'on' }))).toEqual({ checked: true, toggle: 'radio' });
	});

	test('el indeterminado no tiene marca propia: lleva su dibujo en la columna del icono', () => {
		expect(entryCheck(e({ kind: 'checkmark', state: 'indeterminate' }))).toEqual({
			checked: null,
			toggle: 'checkbox',
			mixedIcon: 'checkbox-mixed-symbolic',
		});
	});

	test('las teclas de cada combinación, una por recuadro', () => {
		expect(shortcutChords([['Control', 'q']])).toEqual([['Ctrl', 'Q']]);
		expect(
			shortcutChords([
				['Control', 'Q'],
				[],
				['Alt', 'F4'],
			])
		).toEqual([
			['Ctrl', 'Q'],
			['Alt', 'F4'],
		]);
		expect(shortcutChords(undefined)).toEqual([]);
	});
});
