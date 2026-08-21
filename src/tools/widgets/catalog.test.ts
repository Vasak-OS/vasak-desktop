import { describe, expect, test } from 'bun:test';
import {
	CELL_GAP,
	CELL_SIZE,
	clampToGrid,
	defaultLayout,
	firstFreeSlot,
	gridSize,
	overlaps,
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
		const hd = gridSize(1920, 1080);
		const cuatroK = gridSize(3840, 2160);

		expect(cuatroK.columns).toBeGreaterThan(hd.columns);
		expect(cuatroK.rows).toBeGreaterThan(hd.rows);
	});

	test('el cálculo de celdas coincide con el tamaño que se dibuja', () => {
		const { columns } = gridSize(1920, 1080);
		const ocupado = columns * CELL_SIZE + (columns - 1) * CELL_GAP;

		expect(ocupado).toBeLessThanOrEqual(1920);
	});

	test('una pantalla diminuta deja al menos una celda, no cero', () => {
		expect(gridSize(100, 100)).toEqual({ columns: 1, rows: 1 });
	});
});

describe('encajar en la cuadrícula', () => {
	test('un widget que se pasa del borde derecho vuelve adentro sin encogerse', () => {
		const encajado = clampToGrid(widget(12, 1, 3, 2), 10, 8);

		expect(encajado.w).toBe(3);
		expect(encajado.x + encajado.w - 1).toBeLessThanOrEqual(10);
	});

	test('un widget más grande que la pantalla se recorta al tamaño posible', () => {
		const encajado = clampToGrid(widget(1, 1, 20, 20), 6, 4);

		expect(encajado).toMatchObject({ x: 1, y: 1, w: 6, h: 4 });
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
		const hueco = firstFreeSlot([widget(1, 1, 3, 3)], { w: 2, h: 2 }, 10, 8);

		expect(hueco).not.toBeNull();
		expect(overlaps({ ...widget(0, 0, 2, 2), ...hueco! }, widget(1, 1, 3, 3))).toBe(false);
	});

	test('si no hay lugar lo dice, en vez de apilar uno encima de otro', () => {
		// Una cuadrícula de 2×2 con un widget de 2×2 adentro está llena.
		expect(firstFreeSlot([widget(1, 1, 2, 2)], { w: 2, h: 2 }, 2, 2)).toBeNull();
	});
});

describe('escritorio nuevo', () => {
	test('arranca con el reloj y la música, como antes de tener cuadrícula', () => {
		const tipos = defaultLayout(false).map((w) => w.type);
		expect(tipos).toEqual(['clock', 'music']);
	});

	test('si la persona tenía los archivos a la vista, aparecen como widget', () => {
		expect(defaultLayout(true).map((w) => w.type)).toContain('files');
	});

	test('ningún widget por omisión se pisa con otro', () => {
		const puestos = defaultLayout(true);

		for (let i = 0; i < puestos.length; i++) {
			for (let j = i + 1; j < puestos.length; j++) {
				expect(overlaps(puestos[i], puestos[j])).toBe(false);
			}
		}
	});
});
