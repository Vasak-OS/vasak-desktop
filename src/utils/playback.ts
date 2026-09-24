/**
 * Las cuentas de la reproducción, aparte de los componentes que las dibujan.
 *
 * MPRIS mide en **microsegundos** y el reproductor sólo manda la posición
 * cuando le preguntan: `Position` es la única propiedad que la especificación
 * exime de avisar cuando cambia. O sea que entre dos sondeos la barra no se
 * mueve sola — se mueve porque acá se calcula cuánto pasó desde la última vez
 * que el reproductor habló.
 */

/** Un microsegundo por millonésima de segundo, escrito una sola vez. */
const POR_SEGUNDO = 1_000_000;

/**
 * Un tiempo de MPRIS como lo lee una persona: `3:07`, y `1:02:30` si pasa de
 * la hora.
 *
 * Lo que no es un número —o es negativo, que pasa con los reproductores que
 * mandan `-1` por «no sé»— vuelve en cero y no en `NaN:aN`.
 */
export function formatDuration(micros: number): string {
	if (!Number.isFinite(micros) || micros <= 0) return '0:00';

	const total = Math.floor(micros / POR_SEGUNDO);
	const segundos = total % 60;
	const minutos = Math.floor(total / 60) % 60;
	const horas = Math.floor(total / 3600);

	const ss = String(segundos).padStart(2, '0');
	if (horas === 0) return `${minutos}:${ss}`;
	return `${horas}:${String(minutos).padStart(2, '0')}:${ss}`;
}

/**
 * Qué fracción de la pista lleva sonando, entre 0 y 1.
 *
 * Sin duración no hay fracción: una radio en vivo no publica `mpris:length`, y
 * una barra llena o vacía diría algo que nadie sabe.
 */
export function progressRatio(position: number, length: number): number {
	if (!Number.isFinite(length) || length <= 0) return 0;
	if (!Number.isFinite(position) || position <= 0) return 0;
	return Math.min(1, position / length);
}

/** La posición que le corresponde a un clic hecho a esta altura de la barra. */
export function positionFromRatio(ratio: number, length: number): number {
	if (!Number.isFinite(length) || length <= 0) return 0;
	const acotada = Math.min(1, Math.max(0, Number.isFinite(ratio) ? ratio : 0));
	return Math.round(acotada * length);
}

/**
 * Dónde va la posición ahora, partiendo de la última que mandó el reproductor.
 *
 * Con la reproducción pausada no avanza: la última que llegó sigue siendo la
 * verdadera. Y nunca pasa del final, porque la pista que termina cambia a la
 * siguiente y hasta que eso llegue la barra se queda donde está en vez de
 * seguir de largo.
 */
export function positionNow(
	base: number,
	elapsedMs: number,
	playing: boolean,
	length: number
): number {
	const desde = Number.isFinite(base) && base > 0 ? base : 0;
	if (!playing) return desde;

	const avanzada = desde + Math.max(0, elapsedMs) * 1000;
	if (Number.isFinite(length) && length > 0) return Math.min(avanzada, length);
	return avanzada;
}

/** Lo que entra en el widget, según el alto que le queda. */
export interface Sections {
	/** La barra de progreso con los tiempos. */
	progress: boolean;
	/** El álbum, debajo del artista. */
	album: boolean;
	/** Volumen, aleatorio y repetición. */
	extras: boolean;
}

/**
 * Qué secciones entran con el alto que hay.
 *
 * El widget se redimensiona en la cuadrícula, así que no hay un tamaño para el
 * que diseñar: hay una escalera. Primero lo que no puede faltar —la portada, el
 * título y el transporte—, y a medida que sobra alto aparecen la barra, el
 * álbum y los controles de sesión. Los cortes están donde la fila siguiente deja
 * de entrar sin apretar a las de arriba.
 *
 * Las medidas son las de la cuadrícula: una celda son 120 px y las filas van
 * separadas por 12, así que un widget de una fila mide 120 y uno de dos, 252.
 * Los cortes están puestos entre esos dos números y no en cualquier lado.
 */
export function sectionsFor(height: number): Sections {
	const alto = Number.isFinite(height) ? height : 0;
	return {
		progress: alto >= 150,
		album: alto >= 190,
		extras: alto >= 230,
	};
}

/**
 * La repetición que sigue a la de ahora.
 *
 * El botón rota entre las tres formas que define MPRIS en el orden en que se
 * usan: sin repetición, repetir la lista, repetir la pista. Lo que llegue y no
 * sea ninguna de las tres —un reproductor puede mandar cualquier cosa— arranca
 * la rueda de nuevo.
 */
export function nextLoop(current: string | null | undefined): 'None' | 'Playlist' | 'Track' {
	switch (current) {
		case 'None':
			return 'Playlist';
		case 'Playlist':
			return 'Track';
		default:
			return 'None';
	}
}
