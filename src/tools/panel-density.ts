/**
 * Cuánto texto entra en el panel (vasak-desktop#151).
 *
 * En una pantalla ancha cada píldora lleva su texto: el título de la canción,
 * el nombre de la red y del auricular, la fecha bajo la hora. En una más
 * angosta esos textos largos se pliegan al icono —el nombre entero queda en el
 * globo y en el nombre accesible— y, más angosta todavía, también los números
 * del volumen, la batería y el clima, que se leen en sus applets. La barra de
 * ventanas no se pliega nunca: ya son sólo iconos. Así ninguna píldora se
 * monta sobre otra.
 *
 * Se decide por el espacio disponible —el viewport, que la capa ancla de borde a
 * borde— medido con `ResizeObserver` sobre `documentElement`. Medir la `<nav>`
 * no sirve: en compacto es `w-fit` y su ancho es el del contenido, con lo que
 * medirla realimenta el plegado (ver `watchPanelDensity`).
 */
export type PanelDensity = 'full' | 'compact' | 'tight';

/** Desde acá, todos los textos. */
export const FULL_FROM = 1400;
/** Desde acá, los números; por debajo, sólo los iconos. */
export const COMPACT_FROM = 960;

export function panelDensity(length: number): PanelDensity {
	if (length >= FULL_FROM) return 'full';
	if (length >= COMPACT_FROM) return 'compact';
	return 'tight';
}

/** Si el texto largo de una píldora (nombres, título, fecha) entra. */
export const showsNames = (density: PanelDensity) => density === 'full';

/** Si entran los números cortos (volumen, batería, grados). */
export const showsNumbers = (density: PanelDensity) => density !== 'tight';
