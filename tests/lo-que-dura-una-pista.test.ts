/**
 * Las cuentas del reproductor, que es donde vive lo que la barra dibuja.
 *
 * Se prueban acá y no montando el widget porque este repositorio todavía no
 * tiene con qué montar, y porque lo que puede salir mal son las cuentas: un
 * `-1` de duración, una pista sin final, una posición que sigue avanzando
 * después de que la música se pausó.
 */
import { describe, expect, test } from 'bun:test';
import {
	formatDuration,
	positionFromRatio,
	positionNow,
	progressRatio,
	sectionsFor,
} from '../src/utils/playback';

const SEGUNDO = 1_000_000;

describe('el tiempo que se muestra', () => {
	test('los minutos llevan el segundo con dos cifras', () => {
		expect(formatDuration(7 * SEGUNDO)).toBe('0:07');
		expect(formatDuration(67 * SEGUNDO)).toBe('1:07');
		expect(formatDuration(600 * SEGUNDO)).toBe('10:00');
	});

	test('pasada la hora aparece la hora, y el minuto pasa a dos cifras', () => {
		expect(formatDuration(3600 * SEGUNDO)).toBe('1:00:00');
		expect(formatDuration(3750 * SEGUNDO)).toBe('1:02:30');
	});

	/**
	 * El caso que no es hipotético: varios reproductores mandan `-1` cuando no
	 * saben cuánto dura, y una radio en vivo no manda nada.
	 */
	test('lo que no es una duración vuelve en cero y no en NaN', () => {
		for (const entrada of [0, -1, -1 * SEGUNDO, Number.NaN, Number.POSITIVE_INFINITY]) {
			expect(formatDuration(entrada)).toBe('0:00');
		}
	});
});

describe('la fracción que lleva sonando', () => {
	test('es la parte de la duración, y nunca pasa de uno', () => {
		expect(progressRatio(30 * SEGUNDO, 120 * SEGUNDO)).toBe(0.25);
		expect(progressRatio(200 * SEGUNDO, 120 * SEGUNDO)).toBe(1);
	});

	test('sin duración no hay fracción, en vez de una barra que miente', () => {
		expect(progressRatio(30 * SEGUNDO, 0)).toBe(0);
		expect(progressRatio(30 * SEGUNDO, -1)).toBe(0);
		expect(progressRatio(30 * SEGUNDO, Number.NaN)).toBe(0);
	});

	test('y una posición que no lo es tampoco', () => {
		expect(progressRatio(-5, 120 * SEGUNDO)).toBe(0);
		expect(progressRatio(Number.NaN, 120 * SEGUNDO)).toBe(0);
	});
});

describe('el clic en la barra', () => {
	test('cae donde se apretó', () => {
		expect(positionFromRatio(0.5, 120 * SEGUNDO)).toBe(60 * SEGUNDO);
		expect(positionFromRatio(0, 120 * SEGUNDO)).toBe(0);
		expect(positionFromRatio(1, 120 * SEGUNDO)).toBe(120 * SEGUNDO);
	});

	test('un clic que se fue de la barra se queda en sus bordes', () => {
		expect(positionFromRatio(-0.3, 120 * SEGUNDO)).toBe(0);
		expect(positionFromRatio(1.4, 120 * SEGUNDO)).toBe(120 * SEGUNDO);
	});

	test('sin duración no hay adónde saltar', () => {
		expect(positionFromRatio(0.5, 0)).toBe(0);
	});
});

describe('la posición entre dos sondeos', () => {
	test('avanza con el reloj mientras suena', () => {
		expect(positionNow(10 * SEGUNDO, 1500, true, 120 * SEGUNDO)).toBe(11.5 * SEGUNDO);
	});

	/**
	 * Lo que distingue esta función de una suma: en pausa la última posición que
	 * mandó el reproductor **es** la posición, y sumarle el reloj haría que la
	 * barra siguiera corriendo con la música detenida.
	 */
	test('en pausa se queda donde el reproductor la dejó', () => {
		expect(positionNow(10 * SEGUNDO, 60_000, false, 120 * SEGUNDO)).toBe(10 * SEGUNDO);
	});

	test('no pasa del final aunque el sondeo tarde', () => {
		expect(positionNow(119 * SEGUNDO, 30_000, true, 120 * SEGUNDO)).toBe(120 * SEGUNDO);
	});

	test('sin duración avanza igual, que es lo que hace una radio', () => {
		expect(positionNow(10 * SEGUNDO, 2000, true, 0)).toBe(12 * SEGUNDO);
	});
});

describe('lo que entra en el widget', () => {
	/** Una fila de la cuadrícula: 120 px. Es el tamaño por omisión del widget. */
	test('en el de una fila queda lo que no puede faltar', () => {
		const secciones = sectionsFor(120);
		expect(secciones).toEqual({ progress: false, album: false, extras: false });
	});

	/** Dos filas: 120 + 12 de separación + 120. */
	test('en el de dos filas entra todo', () => {
		const secciones = sectionsFor(252);
		expect(secciones).toEqual({ progress: true, album: true, extras: true });
	});

	test('y entremedio las filas aparecen de a una', () => {
		expect(sectionsFor(160).progress).toBe(true);
		expect(sectionsFor(160).album).toBe(false);
		expect(sectionsFor(200).album).toBe(true);
		expect(sectionsFor(200).extras).toBe(false);
	});

	test('un alto que todavía no se midió no dibuja filas de más', () => {
		expect(sectionsFor(0).progress).toBe(false);
		expect(sectionsFor(Number.NaN).progress).toBe(false);
	});
});
