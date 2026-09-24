import { describe, expect, test } from 'bun:test';
import {
	CELL_GAP,
	CELL_SIZE,
	clampToGrid,
	defaultLayout,
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

	test('arranca con el reloj y la música', () => {
		const types = defaultLayout(false, columns, rows).map((w) => w.type);
		expect(types).toEqual(['clock', 'music']);
	});

	test('si la persona tenía los archivos a la vista, aparecen como widget', () => {
		expect(defaultLayout(true, columns, rows).map((w) => w.type)).toContain('files');
	});

	test('ningún widget por omisión se pisa con otro', () => {
		expect(anyOverlap(defaultLayout(true, columns, rows))).toBe(false);
	});

	/**
	 * El reloj iba en la columna 5 —o la 7 con archivos— y en la fila 3, sin
	 * importar la pantalla: en 14 columnas quedaba flotando a la izquierda del
	 * centro.
	 */
	test('el reloj va arriba y pegado al borde derecho, en cualquier pantalla', () => {
		for (const [width, height] of [
			[1366, 768],
			[1920, 1080],
			[3840, 2160],
		]) {
			const size = gridSize(width, height);
			const clock = defaultLayout(true, size.columns, size.rows).find((w) => w.type === 'clock');

			expect(clock?.y).toBe(1);
			expect((clock?.x ?? 0) + (clock?.w ?? 0) - 1).toBe(size.columns);
		}
	});

	test('la música va debajo del reloj, alineada con él', () => {
		const layout = defaultLayout(false, columns, rows);
		const clock = layout.find((w) => w.type === 'clock');
		const music = layout.find((w) => w.type === 'music');

		expect(music?.x).toBe(clock?.x);
		expect(music?.y).toBe((clock?.y ?? 0) + (clock?.h ?? 0));
	});

	test('los tamaños son los del catálogo, con el reloj de cuatro de ancho', () => {
		const layout = defaultLayout(false, columns, rows);

		expect(layout.find((w) => w.type === 'clock')).toMatchObject(WIDGETS.clock.default);
		expect(layout.find((w) => w.type === 'music')).toMatchObject(WIDGETS.music.default);
		expect(WIDGETS.clock.default.w).toBeGreaterThanOrEqual(4);
	});

	/**
	 * Que no se pisen no alcanza: una disposición con los archivos solos
	 * tampoco se pisa. Con el ancho de siempre, en 7 columnas quedaban dos
	 * libres y el reloj y la música se descartaban sin que nada fallara.
	 */
	test('en una pantalla angosta entran los tres, sin pisarse', () => {
		const layout = defaultLayout(true, 7, 6);

		expect(anyOverlap(layout)).toBe(false);
		expect(layout.map((w) => w.type).sort()).toEqual(['clock', 'files', 'music']);
	});
});

describe('qué disposición se muestra', () => {
	const { columns, rows } = gridSize(1920, 1080);

	test('sin nada guardado, la de siempre', () => {
		const result = resolveLayout(undefined, false, columns, rows);

		expect(result.fromDefault).toBe(true);
		expect(result.placements.map((w) => w.type)).toEqual(['clock', 'music']);
	});

	/**
	 * El caso de la persona que no quiere widgets: sacaba el último, se guardaba
	 * la lista vacía, y al releer la configuración volvían el reloj y la música.
	 */
	test('una lista vacía es un escritorio libre, no uno sin configurar', () => {
		const result = resolveLayout([], true, columns, rows);

		expect(result.placements).toEqual([]);
		expect(result.fromDefault).toBe(false);
	});

	test('lo guardado se respeta', () => {
		const saved = [{ id: 'weather-1', type: 'weather', x: 2, y: 2, w: 3, h: 3 }];
		const result = resolveLayout(saved, true, columns, rows);

		expect(result.placements).toEqual(saved as WidgetPlacement[]);
		expect(result.fromDefault).toBe(false);
	});

	test('un widget que esta versión no conoce se deja afuera', () => {
		const saved = [
			{ id: 'x', type: 'no-existe', x: 1, y: 1, w: 1, h: 1 },
			{ id: 'clock', type: 'clock', x: 1, y: 1, w: 4, h: 2 },
			null,
		];

		expect(resolveLayout(saved, false, columns, rows).placements.map((w) => w.id)).toEqual([
			'clock',
		]);
	});
});

describe('acomodar una disposición entera', () => {
	/**
	 * El caso exacto que encontró la revisión: en 1366×768 la cuadrícula tiene
	 * 10×5, y al acomodar la música para que entre, quedaba encima del reloj.
	 */
	test('en una pantalla chica ningún widget queda encima de otro', () => {
		const { columns, rows } = gridSize(1366, 768);

		for (const withFiles of [false, true]) {
			expect(anyOverlap(fitAll(defaultLayout(withFiles, columns, rows), columns, rows))).toBe(
				false
			);
		}
	});

	test('todos quedan dentro de la cuadrícula', () => {
		const { columns, rows } = gridSize(1366, 768);

		for (const placement of fitAll(defaultLayout(true, columns, rows), columns, rows)) {
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
