import { describe, expect, test } from 'bun:test';
import {
	INNER_EDGE_BAND,
	isNearInnerEdge,
	PANEL_HIDE_TRANSFORM,
	panelHideClass,
	panelRegionMode,
	REVEAL_STRIP_PX,
	readPanelAutohide,
	revealStripRect,
} from './panel-autohide';
import { PANEL_POSITIONS } from './panel-position';

describe('readPanelAutohide', () => {
	test('sólo true lo prende; cualquier otra cosa es apagado', () => {
		// La sección `panel` existe desde los indicadores; el caso normal es que
		// esté y la clave no. El archivo se edita a mano: un `"si"` no es true.
		expect(readPanelAutohide({ panel: { autohide: true } })).toBe(true);
		for (const config of [
			{},
			null,
			{ panel: {} },
			{ panel: { autohide: false } },
			{ panel: { autohide: 'si' } },
			{ panel: { autohide: 1 } },
			{ panel: 'autohide' },
		]) {
			expect(readPanelAutohide(config)).toBe(false);
		}
	});
});

describe('la barra se desliza hacia su borde', () => {
	test('cada lado se esconde hacia el suyo, y a la vista no lleva clase', () => {
		expect(PANEL_HIDE_TRANSFORM.top).toBe('-translate-y-full');
		expect(PANEL_HIDE_TRANSFORM.bottom).toBe('translate-y-full');
		expect(PANEL_HIDE_TRANSFORM.left).toBe('-translate-x-full');
		expect(PANEL_HIDE_TRANSFORM.right).toBe('translate-x-full');
		for (const position of PANEL_POSITIONS) {
			expect(panelHideClass(position, true)).toBe(PANEL_HIDE_TRANSFORM[position]);
			expect(panelHideClass(position, false)).toBe('');
		}
	});
});

describe('la línea de revelado contra el borde', () => {
	test('va pegada al borde dominante y cruza el lado largo', () => {
		const [w, h] = [1920, 38];
		expect(revealStripRect('top', w, h)).toEqual({ x: 0, y: 0, width: w, height: REVEAL_STRIP_PX });
		expect(revealStripRect('bottom', w, h)).toEqual({
			x: 0,
			y: h - REVEAL_STRIP_PX,
			width: w,
			height: REVEAL_STRIP_PX,
		});
		expect(revealStripRect('left', h, 1080)).toEqual({
			x: 0,
			y: 0,
			width: REVEAL_STRIP_PX,
			height: 1080,
		});
		expect(revealStripRect('right', h, 1080)).toEqual({
			x: h - REVEAL_STRIP_PX,
			y: 0,
			width: REVEAL_STRIP_PX,
			height: 1080,
		});
	});

	test('un grosor a medida se respeta', () => {
		expect(revealStripRect('top', 100, 40, 2)).toEqual({ x: 0, y: 0, width: 100, height: 2 });
	});
});

describe('el borde interior, por donde sale el puntero', () => {
	// Franja 1920x38; con el panel arriba el borde interior es abajo (y grande).
	const [w, h] = [1920, 38];
	test('contra el borde interior de cada lado cuenta como saliendo', () => {
		expect(isNearInnerEdge(960, h - 1, 'top', w, h)).toBe(true);
		expect(isNearInnerEdge(960, h - 1 - INNER_EDGE_BAND, 'top', w, h)).toBe(false);
		expect(isNearInnerEdge(960, 1, 'bottom', w, h)).toBe(true);
		expect(isNearInnerEdge(w - 1, 500, 'left', h, 1080)).toBe(true);
		expect(isNearInnerEdge(1, 500, 'right', h, 1080)).toBe(true);
	});

	test('apuntando por el medio de la franja, no está saliendo', () => {
		expect(isNearInnerEdge(960, 18, 'top', w, h)).toBe(false);
		expect(isNearInnerEdge(960, 20, 'bottom', w, h)).toBe(false);
	});
});

describe('qué recibe el puntero en cada estado', () => {
	test('sin auto-ocultar: píldoras, salvo que haya superficie', () => {
		expect(panelRegionMode({ autohide: false, hidden: false, hasSurface: false })).toBe('pills');
		expect(panelRegionMode({ autohide: false, hidden: false, hasSurface: true })).toBe('bar');
	});

	test('con auto-ocultar: la línea si está escondida, la franja entera si está a la vista', () => {
		// Revelada toma la franja entera: no hay huecos (moverse entre píldoras no
		// la esconde) y el «salió» de GTK salta justo al dejar la franja.
		expect(panelRegionMode({ autohide: true, hidden: true, hasSurface: false })).toBe('hidden');
		expect(panelRegionMode({ autohide: true, hidden: false, hasSurface: false })).toBe('surface');
		expect(panelRegionMode({ autohide: true, hidden: true, hasSurface: true })).toBe('hidden');
		expect(panelRegionMode({ autohide: true, hidden: false, hasSurface: true })).toBe('surface');
	});
});
