import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
	BAR_CLASSES,
	GROUP_CLASSES,
	isVertical,
	PANEL_POSITIONS,
	panelPosition,
} from './panel-position';

describe('panelPosition', () => {
	test('sin nada puesto, el panel va arriba', () => {
		// La sección `panel` existe desde antes que esta clave —lleva los
		// interruptores de los indicadores—, así que el caso normal de una
		// instalación vieja es que la sección esté y la clave no.
		expect(panelPosition({})).toBe('top');
		expect(panelPosition(null)).toBe('top');
		expect(panelPosition({ panel: {} })).toBe('top');
		expect(panelPosition({ panel: { weather: false } })).toBe('top');
	});

	test('los cuatro lados se leen', () => {
		for (const side of PANEL_POSITIONS) {
			expect(panelPosition({ panel: { position: side } })).toBe(side);
		}
	});

	test('cualquier otra cosa vale por arriba', () => {
		// El archivo se edita a mano. Con una aserción de tipo, un `"izquierda"`
		// llegaría hasta las clases de la barra y no coincidiría con ninguna: el
		// panel se dibujaría sin acomodo.
		expect(panelPosition({ panel: { position: 'izquierda' } })).toBe('top');
		expect(panelPosition({ panel: { position: 3 } })).toBe('top');
		expect(panelPosition({ panel: 'left' })).toBe('top');
	});
});

describe('isVertical', () => {
	test('a los costados el panel es una columna', () => {
		expect(isVertical('top')).toBe(false);
		expect(isVertical('bottom')).toBe(false);
		expect(isVertical('left')).toBe(true);
		expect(isVertical('right')).toBe(true);
	});
});

describe('las clases de la barra', () => {
	test('cada lado se estira sobre su eje y conserva el grosor', () => {
		// Los 38 píxeles que reserva la superficie son los mismos de los cuatro
		// lados: 36 de la barra (`h-9` / `w-9`) más 2 de margen contra el borde
		// de la pantalla. Lo que cambia es sobre qué eje se estira.
		for (const side of PANEL_POSITIONS) {
			const classes = BAR_CLASSES[side].split(' ');

			if (isVertical(side)) {
				expect(classes).toContain('flex-col');
				expect(classes).toContain('w-9');
				expect(classes).toContain('h-[calc(100vh-8px)]');
			} else {
				expect(classes).toContain('flex-row');
				expect(classes).toContain('h-9');
				expect(classes).toContain('w-[calc(100%-8px)]');
			}
		}
	});

	test('el margen va contra el borde de la pantalla, no contra las ventanas', () => {
		// Que es lo que hace el panel de arriba desde siempre: 2 píxeles arriba y
		// nada abajo, así que la barra queda pegada a lo que haya debajo.
		expect(BAR_CLASSES.top).toContain('mt-0.5');
		expect(BAR_CLASSES.bottom).toContain('mb-0.5');
		expect(BAR_CLASSES.left).toContain('ml-0.5');
		expect(BAR_CLASSES.right).toContain('mr-0.5');
	});
});

describe('la interfaz y el backend leen la misma clave', () => {
	/**
	 * El backend ancla la superficie y la interfaz dibuja lo de adentro, cada
	 * uno leyendo `panel.position` por su cuenta. Si uno de los dos cambia de
	 * clave o de valores, el panel se ancla de un lado y se dibuja para el otro,
	 * y nada falla: queda una columna de iconos acostada en una barra
	 * horizontal.
	 */
	const rust = readFileSync(join(import.meta.dir, '../../src-tauri/src/panel_position.rs'), 'utf8');

	test('la sección y la clave son las mismas', () => {
		expect(rust).toContain('get("panel")');
		expect(rust).toContain('get("position")');
	});

	test('y los cuatro valores también', () => {
		for (const side of PANEL_POSITIONS) {
			expect(rust).toContain(`"${side}" =>`);
		}
	});
});

describe('la barra en píldoras (vasak-desktop#151)', () => {
	test('es una grilla de tres: el centro va al medio y los lados se encogen', () => {
		for (const side of PANEL_POSITIONS) {
			const track = isVertical(side) ? 'grid-rows' : 'grid-cols';
			expect(BAR_CLASSES[side]).toContain(`${track}-[minmax(0,1fr)_auto_minmax(0,1fr)]`);
		}
	});

	test('de costado los grupos se apilan; arriba y abajo van en fila', () => {
		for (const side of PANEL_POSITIONS) {
			const groups = GROUP_CLASSES[side];
			const column = isVertical(side);
			for (const group of [groups.start, groups.center, groups.end]) {
				expect(group.split(' ').includes('flex-col')).toBe(column);
			}
			expect(groups.start).toContain('justify-start');
			expect(groups.center).toContain('justify-center');
			expect(groups.end).toContain('justify-end');
		}
	});
});
