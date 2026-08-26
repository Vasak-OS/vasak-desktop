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
		default: { w: 3, h: 2 },
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
 * Son los dos que ya estaban —reloj y música— en el mismo lugar donde se
 * dibujaban antes, más los archivos si la persona los tenía habilitados: nadie
 * debería perder nada al actualizar.
 */
export function defaultLayout(showFiles: boolean): WidgetPlacement[] {
	// Con los archivos a la izquierda, el reloj y la música se corren a la
	// derecha: en la columna 5 arrancaban justo encima del widget de archivos.
	const columna = showFiles ? 7 : 5;

	const inicial: WidgetPlacement[] = [
		{ id: 'clock', type: 'clock', x: columna, y: 3, w: 3, h: 2 },
		{ id: 'music', type: 'music', x: columna, y: 5, w: 3, h: 2 },
	];

	if (showFiles) {
		inicial.unshift({ id: 'files', type: 'files', x: 1, y: 1, w: 5, h: 6 });
	}

	return inicial;
}

/** Cuántas celdas entran en un área, descontando los márgenes. */
export function gridSize(width: number, height: number) {
	const usable = (total: number) => Math.max(1, total - GRID_PADDING * 2 + CELL_GAP);
	const celdas = (total: number) => Math.max(1, Math.floor(usable(total) / (CELL_SIZE + CELL_GAP)));

	return { columns: celdas(width), rows: celdas(height) };
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
	const ubicados: WidgetPlacement[] = [];

	for (const placement of placements) {
		const acomodado = clampToGrid(placement, columns, rows);

		if (!ubicados.some((otro) => overlaps(acomodado, otro))) {
			ubicados.push(acomodado);
			continue;
		}

		const hueco = firstFreeSlot(ubicados, { w: acomodado.w, h: acomodado.h }, columns, rows);

		if (hueco) {
			ubicados.push({ ...acomodado, ...hueco });
		}
	}

	return ubicados;
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
			const candidato = { id: '', type: 'clock' as WidgetType, x, y, ...size };
			if (!existing.some((otro) => overlaps(candidato, otro))) {
				return { x, y };
			}
		}
	}

	return null;
}
