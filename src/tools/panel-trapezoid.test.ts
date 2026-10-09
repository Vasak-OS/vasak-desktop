/**
 * El recorte en trapecio redondeado (vasak-desktop#219, cantos como RYOKU).
 */
import { describe, expect, test } from 'bun:test';
import { PANEL_POSITIONS } from '@/tools/panel-position';
import { CHAMFER, trapezoidClipPath, WIDE_RADIUS } from '@/tools/panel-trapezoid';

describe('trapezoidClipPath', () => {
	test('sin tamaño no recorta', () => {
		expect(trapezoidClipPath('top', 0, 0)).toBe('none');
		expect(trapezoidClipPath('top', 100, 0)).toBe('none');
	});

	test('arma un path() cerrado con arcos en las cuatro posiciones', () => {
		for (const position of PANEL_POSITIONS) {
			const w = position === 'left' || position === 'right' ? 38 : 900;
			const h = position === 'left' || position === 'right' ? 360 : 38;
			const clip = trapezoidClipPath(position, w, h);
			expect(clip).toStartWith("path('M ");
			expect(clip).toEndWith("Z')");
			// Cuatro arcos: dos cóncavos (parte ancha) y dos convexos (angosta).
			expect(clip.match(/ A /g)?.length).toBe(4);
		}
	});

	test('los arcos de la parte ancha usan el radio del scoop', () => {
		// En `top` los dos primeros arcos (parte ancha, arriba) van con WIDE_RADIUS.
		const clip = trapezoidClipPath('top', 900, 38);
		expect(clip).toContain(`A ${WIDE_RADIUS} ${WIDE_RADIUS} 0 0 1`);
	});

	test('top y bottom son espejo: invierten el sweep de los arcos', () => {
		// La misma forma apoyada contra el borde de arriba o el de abajo recorre el
		// contorno con la orientación invertida, así que el sweep cambia.
		const top = trapezoidClipPath('top', 900, 38);
		const bottom = trapezoidClipPath('bottom', 900, 38);
		expect(top).toContain('0 0 1'); // sweep 1 arriba
		expect(bottom).toContain('0 0 0'); // sweep 0 abajo (espejo)
	});

	test('acota los radios y el bisel a un panel chico sin que los arcos se crucen', () => {
		// Un panel muy cortito no puede llevar el bisel entero; no debe lanzar ni
		// generar coordenadas NaN.
		const clip = trapezoidClipPath('top', 40, 10, { chamfer: CHAMFER });
		expect(clip).not.toContain('NaN');
		expect(clip).toStartWith("path('M ");
	});
});
