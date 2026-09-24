/**
 * Qué widgets existen y cuánto miden.
 *
 * El escritorio dibuja una cuadrícula de celdas de tamaño fijo —no una de N
 * columnas proporcional— por la misma razón que Android: un widget de 2×2 tiene
 * que verse igual en una pantalla de 1080p que en una 4K. Con celdas
 * proporcionales el reloj se vería gigante en un monitor grande y apretado en
 * uno chico.
 */
export const CELL_SIZE = 120;
export const CELL_GAP = 12;

/** El margen que se deja libre en los bordes, para que nada quede pegado. */
export const GRID_PADDING = 24;

export type WidgetType = 'clock' | 'music' | 'weather' | 'files';

export type WidgetPlacement = {
	/** Identidad de esta instancia: hace falta porque puede haber dos del mismo tipo. */
	id: string;
	type: WidgetType;
	/** Columna y fila, empezando en 1 como en CSS grid. */
	x: number;
	y: number;
	/** Cuántas celdas ocupa. */
	w: number;
	h: number;
	/** Algunos widgets se muestran de más de una forma; el clima, por ejemplo. */
	variant?: string;
};

export type WidgetDefinition = {
	type: WidgetType;
	/** Clave de traducción del nombre, para el panel de widgets. */
	labelKey: string;
	descriptionKey: string;
	icon: string;
	default: { w: number; h: number };
	min: { w: number; h: number };
	max: { w: number; h: number };
	/** Variantes disponibles, con la primera como la de siempre. */
	variants?: Array<{ id: string; labelKey: string; size: { w: number; h: number } }>;
	/** Cuántas instancias tiene sentido tener. El reloj, una; las notas, varias. */
	unique: boolean;
};

export const WIDGETS: Record<WidgetType, WidgetDefinition> = {
	clock: {
		type: 'clock',
		labelKey: 'widgets.clock.name',
		descriptionKey: 'widgets.clock.description',
		icon: 'clock-symbolic',
		// Cuatro de ancho: con tres, la hora en grande no entraba y se cortaba.
		default: { w: 4, h: 2 },
		min: { w: 2, h: 1 },
		max: { w: 6, h: 3 },
		unique: true,
	},
	music: {
		type: 'music',
		labelKey: 'widgets.music.name',
		descriptionKey: 'widgets.music.description',
		icon: 'multimedia-player-symbolic',
		// Una sola fila alcanza: portada, título y los tres botones entran de
		// lado. Antes el mínimo era de dos filas y por eso no se podía achicar
		// —y el ancho mínimo de dos celdas dejaba los botones sin lugar—.
		default: { w: 4, h: 1 },
		min: { w: 3, h: 1 },
		max: { w: 8, h: 3 },
		unique: true,
	},
	weather: {
		type: 'weather',
		labelKey: 'widgets.weather.name',
		descriptionKey: 'widgets.weather.description',
		icon: 'weather-few-clouds-symbolic',
		default: { w: 3, h: 3 },
		min: { w: 2, h: 1 },
		max: { w: 5, h: 4 },
		// El extendido es el que ya existía; el del día es el resumen de hoy.
		variants: [
			{ id: 'extended', labelKey: 'widgets.weather.extended', size: { w: 3, h: 3 } },
			{ id: 'today', labelKey: 'widgets.weather.today', size: { w: 2, h: 1 } },
		],
		unique: true,
	},
	files: {
		type: 'files',
		labelKey: 'widgets.files.name',
		descriptionKey: 'widgets.files.description',
		icon: 'user-desktop-symbolic',
		default: { w: 5, h: 5 },
		min: { w: 2, h: 2 },
		max: { w: 12, h: 10 },
		unique: true,
	},
};

/**
 * Con qué widgets arranca un escritorio que nunca se configuró.
 *
 * El reloj y la música van **arriba a la derecha**, calculado contra la
 * cuadrícula que hay. Antes iban en columnas fijas —la 5, o la 7 con los
 * archivos—, que en una pantalla de 14 columnas es un reloj flotando en el
 * medio, a la izquierda del centro. Los archivos, si la persona los tenía a la
 * vista, ocupan la izquierda, que es donde se buscan.
 *
 * Los tamaños son los del catálogo, no unos propios: la música iba de 3×2
 * cuando su tamaño de siempre es 4×1.
 */
export function defaultLayout(
	showFiles: boolean,
	columns: number,
	rows: number
): WidgetPlacement[] {
	const clockSize = WIDGETS.clock.default;
	const musicSize = WIDGETS.music.default;
	const column = Math.max(1, columns - Math.max(clockSize.w, musicSize.w) + 1);

	const initial: WidgetPlacement[] = [
		{ id: 'clock', type: 'clock', x: column, y: 1, ...clockSize },
		{ id: 'music', type: 'music', x: column, y: 1 + clockSize.h, ...musicSize },
	];

	if (showFiles) {
		initial.unshift({
			id: 'files',
			type: 'files',
			x: 1,
			y: 1,
			// Angostos hasta donde empieza la columna de la derecha: con el ancho
			// de siempre, en una pantalla chica los archivos dejaban dos columnas
			// libres y el reloj y la música se descartaban por no entrar.
			w: Math.max(WIDGETS.files.min.w, Math.min(WIDGETS.files.default.w, column - 1)),
			h: Math.min(6, rows),
		});
	}

	// En una pantalla angosta los archivos y la columna de la derecha se tocan:
	// `fitAll` manda al que sobra al primer hueco en vez de dejarlo encima.
	return fitAll(initial, columns, rows);
}

/**
 * Qué disposición toca mostrar, a partir de lo guardado.
 *
 * La distinción que importa es entre **nada guardado** y **una lista vacía**.
 * Nada guardado es un escritorio que nunca se configuró, y arranca con la
 * disposición de siempre. Una lista vacía es alguien que sacó todos los
 * widgets: el escritorio queda libre. Antes las dos cosas iban por el mismo
 * lado —`length > 0`—, así que sacar el último widget lo devolvía todo en
 * cuanto se releía la configuración.
 *
 * `fromDefault` dice cuál de las dos fue: la de siempre se recalcula cuando
 * cambia la pantalla, porque no es de nadie; la guardada se acomoda y se
 * respeta.
 */
export function resolveLayout(
	saved: unknown,
	showFiles: boolean,
	columns: number,
	rows: number
): { placements: WidgetPlacement[]; fromDefault: boolean } {
	if (!Array.isArray(saved)) {
		return { placements: defaultLayout(showFiles, columns, rows), fromDefault: true };
	}

	const known = saved.filter(
		(placement): placement is WidgetPlacement =>
			typeof placement?.type === 'string' && Object.hasOwn(WIDGETS, placement.type)
	);

	return { placements: fitAll(known, columns, rows), fromDefault: false };
}

/** Cuántas celdas entran en un área, descontando los márgenes. */
export function gridSize(width: number, height: number) {
	const usable = (total: number) => Math.max(1, total - GRID_PADDING * 2 + CELL_GAP);
	const cells = (total: number) => Math.max(1, Math.floor(usable(total) / (CELL_SIZE + CELL_GAP)));

	return { columns: cells(width), rows: cells(height) };
}

/** Mete una posición adentro de la cuadrícula, sin cambiarle el tamaño. */
export function clampToGrid(
	placement: WidgetPlacement,
	columns: number,
	rows: number
): WidgetPlacement {
	const w = Math.min(placement.w, columns);
	const h = Math.min(placement.h, rows);

	return {
		...placement,
		w,
		h,
		x: Math.min(Math.max(1, placement.x), Math.max(1, columns - w + 1)),
		y: Math.min(Math.max(1, placement.y), Math.max(1, rows - h + 1)),
	};
}

/**
 * Acomoda una disposición entera a la cuadrícula, sin dejar widgets pisados.
 *
 * Acomodar de a uno no alcanza, y el caso lo mostró la revisión del PR: en
 * 1366×768 la cuadrícula tiene 10 columnas y 5 filas; el reloj ocupa las filas
 * 3 y 4, y la música —que estaba en la fila 5 con dos de alto— se acomoda a la
 * fila 4 para entrar, quedando encima del reloj. Y esa disposición pisada se
 * guardaba.
 *
 * Acá cada widget se acomoda y, si cae sobre otro ya ubicado, se lo manda al
 * primer hueco libre. El que no tiene dónde entrar se descarta: es una pantalla
 * donde no cabe, y dejarlo invisible debajo de otro es peor que no tenerlo.
 */
export function fitAll(
	placements: WidgetPlacement[],
	columns: number,
	rows: number
): WidgetPlacement[] {
	const placed: WidgetPlacement[] = [];

	for (const placement of placements) {
		const fitted = clampToGrid(placement, columns, rows);

		if (!placed.some((other) => overlaps(fitted, other))) {
			placed.push(fitted);
			continue;
		}

		const slot = firstFreeSlot(placed, { w: fitted.w, h: fitted.h }, columns, rows);

		if (slot) {
			placed.push({ ...fitted, ...slot });
		}
	}

	return placed;
}

/** Si dos widgets se pisan. */
export function overlaps(a: WidgetPlacement, b: WidgetPlacement): boolean {
	return a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y;
}

/**
 * El primer hueco donde entra un widget de este tamaño, recorriendo de arriba
 * hacia abajo y de izquierda a derecha.
 *
 * Devuelve `null` si no hay lugar: el panel de widgets lo usa para no ofrecer
 * agregar algo que no va a caber.
 */
export function firstFreeSlot(
	existing: WidgetPlacement[],
	size: { w: number; h: number },
	columns: number,
	rows: number
): { x: number; y: number } | null {
	for (let y = 1; y <= rows - size.h + 1; y++) {
		for (let x = 1; x <= columns - size.w + 1; x++) {
			const candidate = { id: '', type: 'clock' as WidgetType, x, y, ...size };
			if (!existing.some((other) => overlaps(candidate, other))) {
				return { x, y };
			}
		}
	}

	return null;
}
