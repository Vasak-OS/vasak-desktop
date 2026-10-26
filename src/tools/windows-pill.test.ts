import { describe, expect, test } from 'bun:test';
import {
	scrollWindowsPillOnWheel,
	WINDOWS_PILL_CAP_REM,
	WINDOWS_PILL_VISIBLE,
	windowsPillCapStyle,
	windowsPillClasses,
} from './windows-pill';

describe('el tope de la píldora de ventanas', () => {
	test('entran tres iconos, salidos de las medidas reales y no de una clave mágica', () => {
		expect(WINDOWS_PILL_VISIBLE).toBe(3);
		// 3·2 (botón) + 2·0.125 (gap) + 2·0.25 (relleno) + 2·0.0625 (borde) = 6.875rem.
		expect(WINDOWS_PILL_CAP_REM).toBeCloseTo(6.875, 5);
	});
});

describe('las clases de desplazamiento según el lado y la densidad', () => {
	test('de costado: columna que desplaza hacia abajo, sin tope de ancho', () => {
		const classes = windowsPillClasses(true, 'compact');
		expect(classes).toContain('overflow-y-auto');
		expect(classes).toContain('overflow-x-hidden');
		expect(classes).toContain('flex-1');
		expect(classes).not.toContain('max-w-[20rem]');
		expect(classes).not.toContain('shrink-0');
	});

	test('horizontal compacto: reserva su ancho con `shrink-0` y desplaza a lo largo', () => {
		const classes = windowsPillClasses(false, 'compact');
		expect(classes).toContain('overflow-x-auto');
		expect(classes).toContain('overflow-y-hidden');
		expect(classes).toContain('shrink-0');
		// Ni el tope distribuido ni el viejo `max-w-[90vw]`: el tope va por estilo.
		expect(classes).not.toContain('max-w-[20rem]');
		expect(classes).not.toContain('max-w-[90vw]');
	});

	test('horizontal distribuido: cede a las vecinas hasta `max-w-[20rem]`, sin `shrink-0`', () => {
		const classes = windowsPillClasses(false, 'distributed');
		expect(classes).toContain('overflow-x-auto');
		expect(classes).toContain('max-w-[20rem]');
		expect(classes).not.toContain('shrink-0');
		expect(classes).not.toContain('max-w-[90vw]');
	});
});

describe('el tope absoluto va por estilo en línea', () => {
	const cap = `${WINDOWS_PILL_CAP_REM}rem`;

	test('de costado topa el alto a tres iconos', () => {
		expect(windowsPillCapStyle(true, 'compact')).toEqual({ maxHeight: cap });
		expect(windowsPillCapStyle(true, 'distributed')).toEqual({ maxHeight: cap });
	});

	test('horizontal compacto topa el ancho a tres iconos', () => {
		expect(windowsPillCapStyle(false, 'compact')).toEqual({ maxWidth: cap });
	});

	test('horizontal distribuido no pone tope en línea: lo pone `max-w-[20rem]`', () => {
		expect(windowsPillCapStyle(false, 'distributed')).toEqual({});
	});
});

describe('la rueda desplaza la píldora en su eje', () => {
	/** Un carril que topa como el real: el desplazamiento queda entre 0 y `max`. */
	const rail = (max: number) => {
		let left = 0;
		let top = 0;
		const clamp = (v: number) => Math.min(Math.max(v, 0), max);
		return {
			get scrollLeft() {
				return left;
			},
			set scrollLeft(v: number) {
				left = clamp(v);
			},
			get scrollTop() {
				return top;
			},
			set scrollTop(v: number) {
				top = clamp(v);
			},
		};
	};

	test('en horizontal, la rueda vertical mueve scrollLeft y no scrollTop', () => {
		const node = { scrollLeft: 10, scrollTop: 5 };
		expect(scrollWindowsPillOnWheel(node, false, 40, 0)).toBe(true);
		expect(node).toEqual({ scrollLeft: 50, scrollTop: 5 });
	});

	test('de costado, la rueda vertical mueve scrollTop y no scrollLeft', () => {
		const node = { scrollLeft: 10, scrollTop: 5 };
		expect(scrollWindowsPillOnWheel(node, true, 40, 0)).toBe(true);
		expect(node).toEqual({ scrollLeft: 10, scrollTop: 45 });
	});

	test('usa deltaX cuando deltaY es 0 (rueda horizontal)', () => {
		const node = { scrollLeft: 0, scrollTop: 0 };
		expect(scrollWindowsPillOnWheel(node, false, 0, 30)).toBe(true);
		expect(node.scrollLeft).toBe(30);
	});

	test('deltaY manda sobre deltaX cuando vienen los dos', () => {
		const node = { scrollLeft: 0, scrollTop: 0 };
		scrollWindowsPillOnWheel(node, false, 20, 99);
		expect(node.scrollLeft).toBe(20);
	});

	test('un monto de 0 no mueve nada y devuelve false', () => {
		const node = { scrollLeft: 7, scrollTop: 7 };
		expect(scrollWindowsPillOnWheel(node, false, 0, 0)).toBe(false);
		expect(scrollWindowsPillOnWheel(node, true, 0, 0)).toBe(false);
		expect(node).toEqual({ scrollLeft: 7, scrollTop: 7 });
	});

	test('en el tope no se mueve y devuelve false, para dejar pasar la rueda', () => {
		const node = rail(100);
		node.scrollLeft = 100;
		expect(scrollWindowsPillOnWheel(node, false, 50, 0)).toBe(false);
		expect(scrollWindowsPillOnWheel(node, false, -50, 0)).toBe(true);
		expect(node.scrollLeft).toBe(50);
	});

	test('sin desbordamiento no se mueve y devuelve false', () => {
		const node = rail(0);
		expect(scrollWindowsPillOnWheel(node, false, 50, 0)).toBe(false);
		expect(scrollWindowsPillOnWheel(node, true, 50, 0)).toBe(false);
	});
});
