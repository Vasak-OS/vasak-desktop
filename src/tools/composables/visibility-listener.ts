/**
 * Un escucha de `visibilitychange` que se puede soltar.
 *
 * Vive aparte porque el error está justo acá y no se ve mirando el código: con un
 * booleano marcando «ya enganché», al desmontarse el último consumidor se
 * limpiaba el temporizador y **el escucha quedaba enganchado para siempre**. Un
 * ciclo de esconder y mostrar la ventana seguía disparando trabajo —un IPC, y a
 * veces un pedido a la red— sin que quedara nadie mirando el dato.
 *
 * Recibe el `document` en lugar de tomarlo del entorno para poder probarlo, y
 * porque en el arranque de una ventana de Tauri no siempre hay uno.
 */
export interface ObservableDocument {
	hidden: boolean;
	addEventListener(type: string, listener: () => void): void;
	removeEventListener(type: string, listener: () => void): void;
}

export interface VisibilityListener {
	/** Engancha, si no estaba enganchado. Llamarlo dos veces no duplica nada. */
	attach(): void;
	/** Suelta. Llamarlo sin haber enganchado no hace nada. */
	detach(): void;
	/** Si está enganchado ahora mismo. */
	attached(): boolean;
}

export function createVisibilityListener(
	doc: ObservableDocument | undefined,
	onVisible: () => void
): VisibilityListener {
	let listener: (() => void) | null = null;

	return {
		attach() {
			if (listener || !doc) return;
			listener = () => {
				if (!doc.hidden) onVisible();
			};
			doc.addEventListener('visibilitychange', listener);
		},
		detach() {
			if (!listener || !doc) return;
			doc.removeEventListener('visibilitychange', listener);
			listener = null;
		},
		attached() {
			return listener !== null;
		},
	};
}
