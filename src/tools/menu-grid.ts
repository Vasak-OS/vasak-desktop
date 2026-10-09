/**
 * Las cuentas del paginado de la variante en grilla del menú (bug #206).
 *
 * Cuántas columnas y filas de mosaicos entran en un área, y de ahí el tamaño de
 * página, salen acá —aparte del componente— para poder probarse sin montar nada
 * ni medir el DOM. El componente sólo mide el área con un `ResizeObserver` y le
 * pasa el ancho y el alto.
 */

export interface GridTracks {
	columns: number;
	rows: number;
	/** Mosaicos por página: `columns * rows`, al menos uno. */
	pageSize: number;
}

/**
 * Columnas y filas que entran en `width × height`, con mosaicos de `colMin × rowH`
 * separados por `gap` (todo en px). Siempre al menos una de cada: un área de alto
 * cero —el primer cuadro, antes de que el observador mida— no deja la página en
 * cero.
 */
export function gridTracks(
	width: number,
	height: number,
	colMin: number,
	rowH: number,
	gap: number
): GridTracks {
	const columns = Math.max(1, Math.floor((width + gap) / (colMin + gap)));
	const rows = Math.max(1, Math.floor((height + gap) / (rowH + gap)));
	return { columns, rows, pageSize: Math.max(1, columns * rows) };
}

/** Cuántas páginas hacen falta para `total` elementos de a `pageSize`. */
export function pageCount(total: number, pageSize: number): number {
	return Math.max(1, Math.ceil(total / Math.max(1, pageSize)));
}

/** Acota una página al rango `[0, count-1]`. */
export function clampPage(page: number, count: number): number {
	return Math.min(Math.max(0, page), Math.max(0, count - 1));
}
