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
		// Las dos esquinas del lado ancho tienen que caer sobre el borde de la
		// pantalla: en `top` en y=0, en `bottom` en y=h, en `left` en x=0 y en
		// `right` en x=w. Chequear sólo el conteo de comandos no lo garantiza: un
		// error en la transformación de left/right puede dejar el ancho contra el
		// borde equivocado sin cambiar nada más.
		const S = 38; // grosor del panel horizontal / ancho del vertical
		const long = 900;
		const anchoEn: Record<string, [string, string]> = {
			top: [`M 0,0`, `${long},0`], // (0,0) y (long,0)
			bottom: [`M 0,${S}`, `${long},${S}`], // (0,h) y (long,h)
			left: [`M 0,0`, `0,${long}`], // (0,0) y (0,long)
			right: [`M ${S},0`, `${S},${long}`], // (w,0) y (w,long)
		};
		for (const position of PANEL_POSITIONS) {
			const w = position === 'left' || position === 'right' ? S : long;
			const h = position === 'left' || position === 'right' ? long : S;
			const clip = trapezoidClipPath(position, w, h);
			const [inicio, otra] = anchoEn[position] as [string, string];
			expect(clip).toContain(inicio); // una esquina ancha, el arranque
			expect(clip).toContain(otra); // la otra esquina ancha, sobre el mismo borde
		}
		expect(trapezoidClipPath('top', 900, S)).not.toBe(trapezoidClipPath('bottom', 900, S));
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
