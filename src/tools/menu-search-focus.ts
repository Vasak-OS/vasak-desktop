/**
 * Dejar el menú listo para escribir cada vez que se abre.
 *
 * La superficie del menú se **esconde**, no se destruye (es un applet, y
 * `anchored_applet.rs` los esconde a propósito, para no recargar la página en
 * cada apertura). Así que la vista se monta una sola vez en la vida
 * del proceso, y todo lo que dependa de un montaje corre una sola vez: el
 * `autofocus` del campo de la librería enfocaba la primera apertura y ninguna
 * de las siguientes.
 *
 * Lo que había en su lugar para las reaperturas era una búsqueda del campo por
 * un `id` que dejó de existir cuando el campo pasó a ser el `SearchField` de
 * `vue-libvasak`. No fallaba: devolvía nada, el `?.` se lo tragaba y el menú
 * abría mudo, sin una línea de log.
 *
 * Vive acá y no adentro del `.vue` porque este repositorio no tiene con qué
 * montar componentes, y lo que se rompió es justo la lógica: que esto se llame
 * desde el aviso `shown` del applet —lo único que sí corre en cada apertura— y
 * no desde un gancho de montaje. (Antes era el evento de foco de la ventana de
 * Tauri; dentro de una superficie de capa esa ventana es la vacía y el evento
 * no llega nunca.)
 */

/** Lo mínimo que este módulo necesita del campo de búsqueda. */
export interface FocusableSearchField {
	/**
	 * Pide el foco y devuelve si llegó.
	 *
	 * El nombre es el que expone `SearchField` de `vue-libvasak` desde la 2.0.0
	 * (`enfocar()` en la 1.x, que queda como alias obsoleto hasta la 3.0). Lo
	 * que se describe acá es el contrato mínimo, para no atar esto a la
	 * librería entera ni al DOM.
	 */
	focus(): boolean;
}

/**
 * Un intento de foco en curso, para poder cortarlo.
 *
 * Mientras se insiste hay un temporizador vivo, y en esos milisegundos el menú
 * puede cerrarse o el foco irse a otra parte: un reintento que llega tarde le
 * robaría el foco a donde el usuario ya está. Se corta al cerrar, al desmontar,
 * y antes de empezar un intento nuevo.
 */
export interface SearchFocusAttempt {
	/** Corta lo que quede pendiente. Llamarlo dos veces no hace nada. */
	cancel(): void;
}

/** `setTimeout`, con su forma de cancelar. Se inyecta para poder probarlo. */
export type Schedule = (run: () => void, ms: number) => () => void;

export interface FocusMenuSearchOptions {
	/**
	 * El campo, pedido en el momento y no recibido una vez.
	 *
	 * El `ref` de la plantilla está vacío hasta que Vue monta, y el evento de
	 * foco de la ventana puede llegar antes.
	 */
	field(): FocusableSearchField | null | undefined;
	/** `setTimeout`, aparte para poder probar los reintentos sin esperarlos. */
	schedule?: Schedule;
	/** Cuántas veces más insistir si el foco no llegó. */
	retries?: number;
	/** Cuánto esperar antes de cada reintento. */
	retryDelayMs?: number;
}

export interface MenuSearchOptions extends FocusMenuSearchOptions {
	/**
	 * Vacía lo que quedó escrito en la apertura anterior.
	 *
	 * Del mismo esconder y mostrar: el filtro es estado de la vista y sobrevive
	 * al cierre, así que el menú reaparecía con la búsqueda vieja y ponerse a
	 * escribir concatenaba contra ella.
	 */
	clear(): void;
}

/**
 * El WebView puede no tener todavía el foco del documento en el instante en que
 * el compositor se lo da a la ventana. `focus()` dice si llegó, así que se
 * insiste sólo mientras no haya llegado, y con un tope: sin él, un campo que
 * nunca acepta dejaría un temporizador rebotando para siempre.
 */
export const DEFAULT_RETRIES = 3;
export const DEFAULT_RETRY_DELAY_MS = 50;

const defaultSchedule: Schedule = (run, ms) => {
	const id = setTimeout(run, ms);
	return () => clearTimeout(id);
};

/**
 * Enfoca el campo, insistiendo mientras el foco no llegue.
 *
 * Sin vaciar nada: esto es lo que se llama cuando el campo se habilita con el
 * menú ya abierto, y ahí el filtro es lo que el usuario está escribiendo.
 */
export function focusMenuSearch(options: FocusMenuSearchOptions): SearchFocusAttempt {
	const {
		field,
		schedule = defaultSchedule,
		retries = DEFAULT_RETRIES,
		retryDelayMs = DEFAULT_RETRY_DELAY_MS,
	} = options;

	let cancelled = false;
	let cancelPending: (() => void) | null = null;
	let left = retries;

	const attempt = () => {
		cancelPending = null;
		if (cancelled) return;
		if (field()?.focus()) return;
		if (left <= 0) return;
		left -= 1;
		cancelPending = schedule(attempt, retryDelayMs);
	};
	attempt();

	return {
		cancel() {
			cancelled = true;
			cancelPending?.();
			cancelPending = null;
		},
	};
}

/** Vacía la búsqueda anterior y enfoca: lo que hace falta al abrir el menú. */
export function prepareMenuSearch(options: MenuSearchOptions): SearchFocusAttempt {
	const { clear, ...focusOptions } = options;
	clear();
	return focusMenuSearch(focusOptions);
}
