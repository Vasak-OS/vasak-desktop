/**
 * De qué lado de la pantalla va el panel.
 *
 * Sale de `panel.position` en `~/.config/vasak/vasak.conf`, la misma sección
 * que dice qué indicadores muestra. Los cuatro lados valen; cualquier otra cosa
 * —o que no diga nada— es arriba, que es donde el panel estuvo siempre.
 *
 * El backend lee esta misma clave con el mismo criterio para anclar la
 * superficie (`panel_position.rs`). Acá se decide **cómo se acomoda lo de
 * adentro**: a los costados el panel es una columna de 38 píxeles de ancho, y
 * una fila de iconos no entra de costado.
 *
 * Vive aparte de las vistas para poder probarlo: importar una vista arrastra
 * Vue, el enrutador y el backend de Tauri, y este repositorio no tiene con qué
 * montar componentes.
 */

/** Los cuatro lados donde puede quedar el panel. */
export const PANEL_POSITIONS = ['top', 'bottom', 'left', 'right'] as const;

export type PanelPosition = (typeof PANEL_POSITIONS)[number];

/** Arriba, que es donde estuvo siempre y donde la gente lo busca. */
export const DEFAULT_PANEL_POSITION: PanelPosition = 'top';

export function isPanelPosition(value: unknown): value is PanelPosition {
	return typeof value === 'string' && (PANEL_POSITIONS as readonly string[]).includes(value);
}

/**
 * La posición que declara una configuración ya leída.
 *
 * Se comprueba el valor en lugar de afirmarlo con una aserción: el archivo se
 * edita a mano, y ahí un `"izquierda"` no es ninguno de los cuatro lados. Con
 * una aserción ese valor llegaría hasta las clases del panel y no coincidiría
 * con ninguna: la barra quedaría sin acomodo.
 */
export function panelPosition(config: unknown): PanelPosition {
	const section =
		config && typeof config === 'object'
			? ((config as Record<string, unknown>).panel as Record<string, unknown> | undefined)
			: undefined;
	const value = section?.position;
	return isPanelPosition(value) ? value : DEFAULT_PANEL_POSITION;
}

/** A los costados el panel es una columna. */
export function isVertical(position: PanelPosition): boolean {
	return position === 'left' || position === 'right';
}

/**
 * Cómo se dibuja la barra en cada lado.
 *
 * El grosor es siempre el mismo —36 píxeles de la barra más los 2 del margen
 * contra el borde de la pantalla, que son los 38 que la superficie reserva—; lo
 * que cambia es sobre qué eje se estira y contra qué borde se apoya.
 *
 * La barra es una grilla de tres (vasak-desktop#151): el principio y el final
 * se reparten lo que sobra y el centro va siempre al medio, aunque un lado
 * tenga más píldoras que el otro. Los lados se encogen (`minmax(0, 1fr)`) para
 * que un nombre de red largo se corte adentro de su píldora en vez de empujar
 * al reloj.
 *
 * El largo se mide en `vh` y no en `%`: ni `html`, ni `body`, ni `#app` tienen
 * alto declarado, así que un porcentaje de alto no resuelve contra nada y la
 * columna se encoge hasta el tamaño de los iconos.
 */
export const BAR_CLASSES: Record<PanelPosition, string> = {
	top: 'w-[calc(100%-8px)] h-9 mx-1 mt-0.5 px-1 flex-row grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]',
	bottom:
		'w-[calc(100%-8px)] h-9 mx-1 mb-0.5 px-1 flex-row grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]',
	left: 'h-[calc(100vh-8px)] w-9 my-1 ml-0.5 py-1 flex-col grid-rows-[minmax(0,1fr)_auto_minmax(0,1fr)] justify-items-center',
	right:
		'h-[calc(100vh-8px)] w-9 my-1 mr-0.5 py-1 flex-col grid-rows-[minmax(0,1fr)_auto_minmax(0,1fr)] justify-items-center',
};

/** Cómo se acomoda cada uno de los tres grupos de píldoras. */
export interface GroupClasses {
	start: string;
	center: string;
	end: string;
}

const ROW: GroupClasses = {
	start: 'justify-start',
	center: 'justify-center',
	end: 'justify-end',
};

const COLUMN: GroupClasses = {
	start: 'flex-col justify-start min-h-0',
	center: 'flex-col justify-center',
	end: 'flex-col justify-end min-h-0',
};

/** De costado los grupos son columnas; arriba y abajo, filas. */
export const GROUP_CLASSES: Record<PanelPosition, GroupClasses> = {
	top: ROW,
	bottom: ROW,
	left: COLUMN,
	right: COLUMN,
};
