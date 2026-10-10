/**
 * El recorte en trapecio invertido (vasak-desktop#219): ancho arriba contra la
 * pantalla con saliente cóncavo, angosto abajo con esquinas convexas.
 */
import { describe, expect, test } from 'bun:test';
import { PANEL_POSITIONS } from '@/tools/panel-position';
import { CHAMFER, CONVEX, FLARE, FLARE_DROP, trapezoidClipPath } from '@/tools/panel-trapezoid';

const countCmd = (clip: string, cmd: string) =>
	clip.match(new RegExp(` ${cmd} `, 'g'))?.length ?? 0;

describe('trapezoidClipPath', () => {
	test('las constantes de la forma son positivas y coherentes', () => {
		// El saliente no puede abrirse más de lo que entra la diagonal.
		expect(FLARE).toBeLessThanOrEqual(CHAMFER);
		expect(CONVEX).toBeGreaterThan(0);
		expect(FLARE_DROP).toBeGreaterThan(0);
	});

	test('sin tamaño no recorta', () => {
		expect(trapezoidClipPath('top', 0, 0)).toBe('none');
		expect(trapezoidClipPath('top', 100, 0)).toBe('none');
	});

	test('arma un path() cerrado con curvas en las cuatro posiciones', () => {
		for (const position of PANEL_POSITIONS) {
			const w = position === 'left' || position === 'right' ? 38 : 900;
			const h = position === 'left' || position === 'right' ? 360 : 38;
			const clip = trapezoidClipPath(position, w, h);
			expect(clip).toStartWith("path('M ");
			expect(clip).toEndWith("Z')");
			expect(clip).not.toContain('NaN');
			// Dos salientes cóncavos (cúbicas) arriba y dos esquinas convexas
			// (cuadráticas) abajo, con tres rectas (dos diagonales + el borde angosto).
			expect(countCmd(clip, 'C')).toBe(2);
			expect(countCmd(clip, 'Q')).toBe(2);
			expect(countCmd(clip, 'L')).toBe(3);
		}
	});

	test('el borde ancho se apoya contra la pantalla; el angosto queda adentro', () => {
		// En `top` el lado ancho (las esquinas de la pantalla) va en y=0 y el
		// angosto en y=h; en `bottom` es al revés (espejo vertical).
		const h = 38;
		const top = trapezoidClipPath('top', 900, h);
		expect(top).toContain('M 0,0'); // esquina ancha contra el borde de arriba
		const bottom = trapezoidClipPath('bottom', 900, h);
		expect(bottom).toContain(`M 0,${h}`); // el lado ancho pasó abajo
		expect(top).not.toBe(bottom);
	});

	test('acota los radios y el bisel a un panel chico sin cruzar las curvas ni dar NaN', () => {
		const clip = trapezoidClipPath('top', 40, 10, { chamfer: CHAMFER });
		expect(clip).not.toContain('NaN');
		expect(clip).toStartWith("path('M ");
	});

	test('respeta los parámetros pasados por opciones', () => {
		const a = trapezoidClipPath('top', 900, 38, { flare: 5, convex: 3 });
		const b = trapezoidClipPath('top', 900, 38, { flare: 18, convex: 10 });
		expect(a).not.toBe(b);
	});
});
