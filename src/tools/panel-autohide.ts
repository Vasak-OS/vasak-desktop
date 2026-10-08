/**
 * El panel que se esconde solo y se revela al pasar el cursor por el borde.
 *
 * Sale de `panel.autohide` en `vasak.conf`, la misma sección que el aspecto. El
 * backend lee esta clave para dejar la zona exclusiva en cero —con auto-ocultar
 * la franja no reserva lugar y las ventanas la ocupan (`panel_autohide.rs`)—; acá
 * vive **cómo** se esconde y se revela: la barra se desliza fuera de la pantalla
 * y la región de entrada se recorta a una línea fina contra el borde, la única
 * parte que sigue recibiendo el puntero. Al tocar esa línea, la barra vuelve.
 *
 * Son funciones puras —leer la clave, la clase que la desliza, el rectángulo de
 * la línea, qué recibe el puntero en cada estado— para poder probarlas sin montar
 * Vue, igual que `panel-appearance.ts`.
 */

import type { InputRect } from '@/services/compositor.service';
import type { PanelPosition } from '@/tools/panel-position';

/** La clave ausente vale por apagado: el panel no se esconde hasta que se pide. */
export function readPanelAutohide(config: unknown): boolean {
	const section =
		config && typeof config === 'object'
			? ((config as Record<string, unknown>).panel as Record<string, unknown> | undefined)
			: undefined;
	return section?.autohide === true;
}

/**
 * La clase que desliza la barra fuera de la pantalla, hacia su borde.
 *
 * Un `translate` del 100 % hacia el borde contra el que se apoya: arriba sube,
 * abajo baja, a los costados va hacia su lado. Vacío cuando está a la vista. Son
 * utilidades base (no `hover:`/`active:`), que la guardia del diseño permite.
 */
export const PANEL_HIDE_TRANSFORM: Record<PanelPosition, string> = {
	top: '-translate-y-full',
	bottom: 'translate-y-full',
	left: '-translate-x-full',
	right: 'translate-x-full',
};

export function panelHideClass(position: PanelPosition, hidden: boolean): string {
	return hidden ? PANEL_HIDE_TRANSFORM[position] : '';
}

/**
 * El grosor de la línea de revelado contra el borde, en píxeles.
 *
 * Fina a propósito: es lo único que sigue recibiendo el puntero con la barra
 * escondida, y todo lo demás de la franja lo atraviesa hasta la ventana de
 * abajo. El borde de la pantalla hace de tope, así que llevar el cursor hasta
 * ahí es fácil aunque la línea sea de pocos píxeles.
 */
export const REVEAL_STRIP_PX = 4;

/**
 * El rectángulo de la línea de revelado, en coordenadas de la superficie.
 *
 * La ventana del panel **es** la franja, así que su viewport mide lo que la
 * superficie: la línea va pegada al borde dominante y cruza todo el lado largo.
 * Es lo que se le pasa al backend como región de entrada mientras la barra está
 * escondida.
 */
export function revealStripRect(
	position: PanelPosition,
	viewportWidth: number,
	viewportHeight: number,
	stripPx: number = REVEAL_STRIP_PX
): InputRect {
	switch (position) {
		case 'top':
			return { x: 0, y: 0, width: viewportWidth, height: stripPx };
		case 'bottom':
			return { x: 0, y: viewportHeight - stripPx, width: viewportWidth, height: stripPx };
		case 'left':
			return { x: 0, y: 0, width: stripPx, height: viewportHeight };
		case 'right':
			return { x: viewportWidth - stripPx, y: 0, width: stripPx, height: viewportHeight };
	}
}

/**
 * Qué parte de la superficie recibe el puntero, según el estado:
 *
 * - `hidden`: sólo la línea del borde (la barra está escondida).
 * - `surface`: la franja entera. Es lo de la barra revelada con auto-ocultar:
 *   así no hay huecos (moverse entre píldoras no la esconde) y, sobre todo, el
 *   `leave-notify` de GTK salta justo cuando el puntero deja la franja —que es
 *   cuando hay que esconderla— y no al cruzar un hueco. Cubre también el mismo
 *   borde que la línea de revelado, así que revelar no parpadea.
 * - `bar`: la barra entera (su recuadro) cuando el tipo dibuja una superficie
 *   continua (flotante, barra, dock), sin auto-ocultar.
 * - `pills`: sólo las píldoras, que es lo de siempre sin auto-ocultar.
 */
/**
 * Cuán cerca del borde interior —el que da al contenido, por donde el puntero
 * sale— cuenta como «saliendo». La franja mide 38 px y las píldoras van en el
 * medio, así que esta banda contra el borde interior queda por fuera de ellas:
 * apuntar a una píldora no la dispara, salir sí.
 */
export const INNER_EDGE_BAND = 6;

/**
 * Si el puntero está contra el borde interior de la franja —el opuesto al borde
 * de la pantalla—, que es por donde sale el puntero al dejar el panel. Es la
 * señal fiable de «saliendo»: el WebView da `pointermove` con coordenadas
 * mientras el puntero está sobre la franja, aunque no dé ningún evento de salida.
 */
export function isNearInnerEdge(
	x: number,
	y: number,
	position: PanelPosition,
	viewportWidth: number,
	viewportHeight: number,
	band: number = INNER_EDGE_BAND
): boolean {
	switch (position) {
		case 'top':
			return y >= viewportHeight - band;
		case 'bottom':
			return y <= band;
		case 'left':
			return x >= viewportWidth - band;
		case 'right':
			return x <= band;
	}
}

export type PanelRegionMode = 'pills' | 'bar' | 'hidden' | 'surface';

export function panelRegionMode(options: {
	autohide: boolean;
	hidden: boolean;
	hasSurface: boolean;
}): PanelRegionMode {
	if (options.autohide) return options.hidden ? 'hidden' : 'surface';
	return options.hasSurface ? 'bar' : 'pills';
}
