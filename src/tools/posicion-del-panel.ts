/**
 * De qué lado de la pantalla va el panel.
 *
 * Sale de `panel.position` en `~/.config/vasak/vasak.conf`, la misma sección
 * que dice qué indicadores muestra. Los cuatro lados valen; cualquier otra cosa
 * —o que no diga nada— es arriba, que es donde el panel estuvo siempre.
 *
 * El backend lee esta misma clave con el mismo criterio para anclar la
 * superficie (`posicion_del_panel.rs`). Acá se decide **cómo se acomoda lo de
 * adentro**: a los costados el panel es una columna de 38 píxeles de ancho, y
 * una fila de iconos no entra de costado.
 *
 * Vive aparte de las vistas para poder probarlo: importar una vista arrastra
 * Vue, el enrutador y el backend de Tauri, y este repositorio no tiene con qué
 * montar componentes.
 */

/** Los cuatro lados donde puede quedar el panel. */
export const POSICIONES_DEL_PANEL = ['top', 'bottom', 'left', 'right'] as const;

export type PosicionDelPanel = (typeof POSICIONES_DEL_PANEL)[number];

/** Arriba, que es donde estuvo siempre y donde la gente lo busca. */
export const POSICION_DEL_PANEL_POR_OMISION: PosicionDelPanel = 'top';

export function esPosicionDelPanel(valor: unknown): valor is PosicionDelPanel {
	return typeof valor === 'string' && (POSICIONES_DEL_PANEL as readonly string[]).includes(valor);
}

/**
 * La posición que declara una configuración ya leída.
 *
 * Se comprueba el valor en lugar de afirmarlo con una aserción: el archivo se
 * edita a mano, y ahí un `"izquierda"` no es ninguno de los cuatro lados. Con
 * una aserción ese valor llegaría hasta las clases del panel y no coincidiría
 * con ninguna: la barra quedaría sin acomodo.
 */
export function posicionDelPanel(config: unknown): PosicionDelPanel {
	const seccion =
		config && typeof config === 'object'
			? ((config as Record<string, unknown>).panel as Record<string, unknown> | undefined)
			: undefined;
	const puesta = seccion?.position;
	return esPosicionDelPanel(puesta) ? puesta : POSICION_DEL_PANEL_POR_OMISION;
}

/** A los costados el panel es una columna. */
export function esVertical(posicion: PosicionDelPanel): boolean {
	return posicion === 'left' || posicion === 'right';
}

/**
 * Cómo se dibuja la barra en cada lado.
 *
 * El grosor es siempre el mismo —36 píxeles de la barra más los 2 del margen
 * contra el borde de la pantalla, que son los 38 que la superficie reserva—; lo
 * que cambia es sobre qué eje se estira y contra qué borde se apoya.
 *
 * El largo se mide en `vh` y no en `%`: ni `html`, ni `body`, ni `#app` tienen
 * alto declarado, así que un porcentaje de alto no resuelve contra nada y la
 * columna se encoge hasta el tamaño de los iconos.
 */
export const CLASES_DE_LA_BARRA: Record<PosicionDelPanel, string> = {
	top: 'w-[calc(100%-8px)] h-9 mx-1 mt-0.5 px-3 flex-row',
	bottom: 'w-[calc(100%-8px)] h-9 mx-1 mb-0.5 px-3 flex-row',
	left: 'h-[calc(100vh-8px)] w-9 my-1 ml-0.5 py-3 flex-col',
	right: 'h-[calc(100vh-8px)] w-9 my-1 mr-0.5 py-3 flex-col',
};
