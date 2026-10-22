import { describe, expect, test } from 'bun:test';
import {
	CELL_GAP,
	CELL_SIZE,
	clampToGrid,
	firstFreeSlot,
	fitAll,
	gridSize,
	overlaps,
	resolveLayout,
	WIDGETS,
	type WidgetPlacement,
} from './catalog';

const widget = (x: number, y: number, w: number, h: number): WidgetPlacement => ({
	id: `${x}-${y}`,
	type: 'clock',
	x,
	y,
	w,
	h,
});

describe('cuadrícula', () => {
	test('las celdas son fijas, así que una pantalla más grande tiene más celdas', () => {
		const fullHd = gridSize(1920, 1080);
		const uhd = gridSize(3840, 2160);

		expect(uhd.columns).toBeGreaterThan(fullHd.columns);
		expect(uhd.rows).toBeGreaterThan(fullHd.rows);
	});

	test('el cálculo de celdas coincide con el tamaño que se dibuja', () => {
		const { columns } = gridSize(1920, 1080);
		const occupied = columns * CELL_SIZE + (columns - 1) * CELL_GAP;

		expect(occupied).toBeLessThanOrEqual(1920);
	});

	test('una pantalla diminuta deja al menos una celda, no cero', () => {
		expect(gridSize(100, 100)).toEqual({ columns: 1, rows: 1 });
	});
});

describe('encajar en la cuadrícula', () => {
	test('un widget que se pasa del borde derecho vuelve adentro sin encogerse', () => {
		const fitted = clampToGrid(widget(12, 1, 3, 2), 10, 8);

		expect(fitted.w).toBe(3);
		expect(fitted.x + fitted.w - 1).toBeLessThanOrEqual(10);
	});

	test('un widget más grande que la pantalla se recorta al tamaño posible', () => {
		const fitted = clampToGrid(widget(1, 1, 20, 20), 6, 4);

		expect(fitted).toMatchObject({ x: 1, y: 1, w: 6, h: 4 });
	});

	test('una posición válida no se toca', () => {
		const original = widget(3, 2, 2, 2);
		expect(clampToGrid(original, 10, 8)).toEqual(original);
	});
});

describe('superposición', () => {
	test('dos widgets pegados no se pisan', () => {
		expect(overlaps(widget(1, 1, 2, 2), widget(3, 1, 2, 2))).toBe(false);
		expect(overlaps(widget(1, 1, 2, 2), widget(1, 3, 2, 2))).toBe(false);
	});

	test('un solapamiento de una sola celda cuenta como pisarse', () => {
		expect(overlaps(widget(1, 1, 2, 2), widget(2, 2, 2, 2))).toBe(true);
	});
});

describe('buscar lugar', () => {
	test('el primer hueco es arriba a la izquierda', () => {
		expect(firstFreeSlot([], { w: 2, h: 2 }, 10, 8)).toEqual({ x: 1, y: 1 });
	});

	test('esquiva lo que ya está puesto', () => {
		const slot = firstFreeSlot([widget(1, 1, 3, 3)], { w: 2, h: 2 }, 10, 8);

		// Se estrecha el tipo con un `throw` en lugar de afirmarlo con `!`: si
		// devolviera null, el test falla acá y dice por qué, en vez de reventar dos
		// líneas más abajo al desarmar un objeto que no existe.
		if (!slot) throw new Error('firstFreeSlot no encontró un slot donde había uno');
		expect(overlaps({ ...widget(0, 0, 2, 2), ...slot }, widget(1, 1, 3, 3))).toBe(false);
	});

	test('si no hay lugar lo dice, en vez de apilar uno encima de otro', () => {
		// Una cuadrícula de 2×2 con un widget de 2×2 adentro está llena.
		expect(firstFreeSlot([widget(1, 1, 2, 2)], { w: 2, h: 2 }, 2, 2)).toBeNull();
	});
});

/** Si algún par de la lista se pisa. */
const anyOverlap = (placements: WidgetPlacement[]) =>
	placements.some((a, i) => placements.slice(i + 1).some((b) => overlaps(a, b)));

describe('escritorio nuevo', () => {
	// 1920×1080: la cuadrícula de 14×7 de una pantalla común.
	const { columns, rows } = gridSize(1920, 1080);

	/**
	 * vasak-desktop#259: el escritorio no trae widgets por omisión, ni el reloj
	 * ni la música ni los archivos. Se suman a mano desde el panel.
	 */
	test('sin nada guardado, no hay ningún widget', () => {
		expect(resolveLayout(undefined, columns, rows)).toEqual([]);
		expect(resolveLayout(null, columns, rows)).toEqual([]);
	});

	test('un valor guardado que no es una lista tampoco trae widgets', () => {
		expect(resolveLayout({ clock: true }, columns, rows)).toEqual([]);
		expect(resolveLayout('clock', columns, rows)).toEqual([]);
	});

	test('el catálogo sigue ofreciendo todos los widgets para sumarlos a mano', () => {
		expect(Object.keys(WIDGETS).sort()).toEqual([
			'calendar',
			'clock',
			'files',
			'music',
			'system',
			'tasks',
			'weather',
		]);
	});

	test('un widget sumado a mano entra en el escritorio libre', () => {
		for (const type of Object.keys(WIDGETS) as Array<keyof typeof WIDGETS>) {
			const { w, h } = WIDGETS[type].default;
			expect(firstFreeSlot([], { w, h }, columns, rows)).toEqual({ x: 1, y: 1 });
		}
	});
});

describe('qué disposición se muestra', () => {
	const { columns, rows } = gridSize(1920, 1080);

	/**
	 * Sacar el último widget guarda una lista vacía: el escritorio queda libre y
	 * lo sigue estando al releer la configuración.
	 */
	test('una lista vacía es un escritorio libre', () => {
		expect(resolveLayout([], columns, rows)).toEqual([]);
	});

	test('lo guardado se respeta', () => {
		const saved = [{ id: 'weather-1', type: 'weather', x: 2, y: 2, w: 3, h: 3 }];

		expect(resolveLayout(saved, columns, rows)).toEqual(saved as WidgetPlacement[]);
	});

	test('quien ya tenía widgets guardados los conserva', () => {
		const saved = [
			{ id: 'clock', type: 'clock', x: 11, y: 1, w: 4, h: 2 },
			{ id: 'music', type: 'music', x: 11, y: 3, w: 4, h: 1 },
		];

		expect(resolveLayout(saved, columns, rows).map((w) => w.type)).toEqual(['clock', 'music']);
	});

	test('un widget que esta versión no conoce se deja afuera', () => {
		const saved = [
			{ id: 'x', type: 'no-existe', x: 1, y: 1, w: 1, h: 1 },
			{ id: 'clock', type: 'clock', x: 1, y: 1, w: 4, h: 2 },
			null,
		];

		expect(resolveLayout(saved, columns, rows).map((w) => w.id)).toEqual(['clock']);
	});
});

describe('acomodar una disposición entera', () => {
	/**
	 * El caso exacto que encontró la revisión: en 1366×768 la cuadrícula tiene
	 * 10×5, y al acomodar la música para que entre, quedaba encima del reloj.
	 */
	test('en una pantalla chica ningún widget queda encima de otro', () => {
		const { columns, rows } = gridSize(1366, 768);
		const saved = [
			{ id: 'clock', type: 'clock', x: 7, y: 3, w: 4, h: 2 },
			{ id: 'music', type: 'music', x: 7, y: 4, w: 4, h: 2 },
		];

		expect(anyOverlap(resolveLayout(saved, columns, rows))).toBe(false);
	});

	test('todos quedan dentro de la cuadrícula', () => {
		const { columns, rows } = gridSize(1366, 768);
		const saved = [
			{ id: 'files', type: 'files', x: 1, y: 1, w: 5, h: 5 },
			{ id: 'clock', type: 'clock', x: 12, y: 1, w: 4, h: 2 },
		];

		for (const placement of resolveLayout(saved, columns, rows)) {
			expect(placement.x + placement.w - 1).toBeLessThanOrEqual(columns);
			expect(placement.y + placement.h - 1).toBeLessThanOrEqual(rows);
		}
	});

	test('lo que no tiene dónde entrar se descarta en vez de quedar tapado', () => {
		const crowded = fitAll([widget(1, 1, 2, 2), widget(1, 1, 2, 2), widget(1, 1, 2, 2)], 2, 2);

		expect(crowded).toHaveLength(1);
	});

	test('una disposición que ya estaba bien no se toca', () => {
		const original = [widget(1, 1, 2, 2), widget(3, 1, 2, 2)];
		expect(fitAll(original, 10, 5)).toEqual(original);
	});
});
