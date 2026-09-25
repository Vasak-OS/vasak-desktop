/**
 * Dejar el menú listo para escribir cada vez que se abre.
 *
 * La ventana del menú se **esconde**, no se destruye (`toggle_menu` en
 * `src-tauri/src/commands/menu.rs` usa `hide()` a propósito, para no recargar la
 * página en cada apertura). Así que la vista se monta una sola vez en la vida
 * del proceso, y todo lo que dependa de un montaje corre una sola vez: el
 * `autofocus` del campo de la librería enfocaba la primera apertura y ninguna
 * de las siguientes.
 *
 * Lo que había en su lugar para las reaperturas era
 * `document.getElementById('search')?.focus()`, con un id que dejó de existir
 * cuando el campo pasó a ser el `SearchField` de `vue-libvasak`. No fallaba: el
 * `?.` se tragaba el `null` y el menú abría sin foco, sin una línea de log.
 *
 * Vive acá y no adentro del `.vue` porque este repositorio no tiene con qué
 * montar componentes, y lo que se rompió es justo la lógica: que esto se llame
 * desde el evento de foco de la ventana —el único que sí corre en cada
 * apertura— y no desde un gancho de montaje.
 */

/** Lo mínimo que este módulo necesita del campo de búsqueda. */
export interface FocusableSearchField {
	/**
	 * Pide el foco y devuelve si llegó.
	 *
	 * El nombre es el que expone `SearchField` de `vue-libvasak`. Lo que se
	 * describe acá es el contrato mínimo, para no atar esto a la librería entera
	 * ni al DOM.
	 */
	enfocar(): boolean;
}

export interface MenuSearchOptions {
	/**
	 * El campo, pedido en el momento y no recibido una vez.
	 *
	 * El `ref` de la plantilla está vacío hasta que Vue monta, y el evento de
	 * foco de la ventana puede llegar antes.
	 */
	field(): FocusableSearchField | null | undefined;
	/**
	 * Vacía lo que quedó escrito en la apertura anterior.
	 *
	 * Del mismo esconder y mostrar: el filtro es estado de la vista y sobrevive
	 * al cierre, así que el menú reaparecía con la búsqueda vieja y ponerse a
	 * escribir concatenaba contra ella.
	 */
	clear(): void;
	/** `setTimeout`, aparte para poder probar los reintentos sin esperarlos. */
	schedule?(run: () => void, ms: number): void;
	/** Cuántas veces más insistir si el foco no llegó. */
	retries?: number;
	/** Cuánto esperar antes de cada reintento. */
	retryDelayMs?: number;
}

/**
 * El WebView puede no tener todavía el foco del documento en el instante en que
 * el compositor se lo da a la ventana. `enfocar()` dice si llegó, así que se
 * insiste sólo mientras no haya llegado, y con un tope: sin él, un campo
 * desactivado —el menú vacío lo desactiva— dejaría un temporizador rebotando
 * para siempre.
 */
export const DEFAULT_RETRIES = 3;
export const DEFAULT_RETRY_DELAY_MS = 50;

export function prepareMenuSearch(options: MenuSearchOptions): void {
	const {
		field,
		clear,
		schedule = (run, ms) => {
			setTimeout(run, ms);
		},
		retries = DEFAULT_RETRIES,
		retryDelayMs = DEFAULT_RETRY_DELAY_MS,
	} = options;

	clear();

	let left = retries;
	const attempt = () => {
		if (field()?.enfocar()) return;
		if (left <= 0) return;
		left -= 1;
		schedule(attempt, retryDelayMs);
	};
	attempt();
}
