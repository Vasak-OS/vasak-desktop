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
export interface DocumentoObservable {
	hidden: boolean;
	addEventListener(tipo: string, manejador: () => void): void;
	removeEventListener(tipo: string, manejador: () => void): void;
}

export interface EscuchaDeVisibilidad {
	/** Engancha, si no estaba enganchado. Llamarlo dos veces no duplica nada. */
	enganchar(): void;
	/** Suelta. Llamarlo sin haber enganchado no hace nada. */
	soltar(): void;
	/** Si está enganchado ahora mismo. */
	enganchado(): boolean;
}

export function crearEscuchaDeVisibilidad(
	documento: DocumentoObservable | undefined,
	alVolverALaVista: () => void
): EscuchaDeVisibilidad {
	let manejador: (() => void) | null = null;

	return {
		enganchar() {
			if (manejador || !documento) return;
			manejador = () => {
				if (!documento.hidden) alVolverALaVista();
			};
			documento.addEventListener('visibilitychange', manejador);
		},
		soltar() {
			if (!manejador || !documento) return;
			documento.removeEventListener('visibilitychange', manejador);
			manejador = null;
		},
		enganchado() {
			return manejador !== null;
		},
	};
}
