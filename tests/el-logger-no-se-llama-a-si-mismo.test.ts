/**
 * El logger se llamaba a sí mismo hasta quedarse sin memoria.
 *
 * `src/utils/logger.ts` reemplaza `console.log`, `console.debug`,
 * `console.warn` y `console.error` en su constructor y reenvía cada llamada al
 * backend con `logFromFrontend`, que es un `invoke`. Sin guardia de reentrada,
 * cualquier `console.*` que ocurra **dentro** del camino de envío vuelve a
 * entrar al logger, y esa vuelta no tiene fondo: la pestaña muere con
 * `V8 javascript OOM (CALL_AND_RETRY_LAST)` antes de dibujar una línea.
 *
 * Eran dos caminos. El aviso del propio `sendLog` cuando el envío falla, que se
 * daba con `console.warn` —la función que el logger acababa de reemplazar— y
 * con el backend caído se realimentaba sola. Y cualquier código que escriba en
 * consola desde adentro de un `invoke`, que cae en lo mismo porque el envío del
 * log es a su vez un `invoke`.
 *
 * Se comprueba con un `logFromFrontend` simulado, contando los intentos. El
 * simulado corta por las suyas a los `MAX_ATTEMPTS`: sin ese tope, una
 * regresión no haría fallar la prueba sino que colgaría el proceso, que es
 * exactamente lo que hacía el escritorio.
 */

import { afterAll, beforeAll, describe, expect, mock, test } from 'bun:test';

/** Dónde corta el simulado para que una regresión falle en vez de colgarse. */
const MAX_ATTEMPTS = 50;

/** Cuántas veces se llamó a `logFromFrontend` desde que se puso a cero. */
let attempts = 0;

/** Qué hace el `logFromFrontend` simulado; cada prueba lo cambia. */
let deliver: () => Promise<unknown> = () => Promise.resolve();

/** Lo que el logger dijo por la consola que guardó al construirse. */
const throughOriginalConsole: string[] = [];

mock.module('@/services/core.service', () => ({
	logFromFrontend: () => {
		attempts += 1;
		if (attempts > MAX_ATTEMPTS) {
			return Promise.resolve();
		}
		return deliver();
	},
	getLogFilePath: () => Promise.reject(new Error('backend caído')),
	readLogFile: () => Promise.reject(new Error('backend caído')),
	getLastLogLines: () => Promise.reject(new Error('backend caído')),
}));

const savedConsole = {
	log: console.log,
	debug: console.debug,
	warn: console.warn,
	error: console.error,
};

/** Deja correr las microtareas del envío antes de mirar el contador. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 50));

let logger: (typeof import('@/utils/logger'))['default'];

beforeAll(async () => {
	// El aviso de fallo sólo se da en desarrollo, que es donde el bucle se
	// destapó.
	process.env.DEV = 'true';
	(globalThis as any).window = { addEventListener: () => {} };

	// La consola que el logger va a guardar como «original» es ésta, así que
	// lo que aparezca acá es lo que salió sin volver a entrar al logger.
	const record = (...args: any[]) => {
		throughOriginalConsole.push(args.map(String).join(' '));
	};
	console.log = record;
	console.debug = record;
	console.warn = record;
	console.error = record;

	logger = (await import('@/utils/logger')).default;
});

afterAll(() => {
	// El logger reemplaza la consola del proceso entero; sin esto se lleva
	// puestos los demás archivos de prueba.
	Object.assign(console, savedConsole);
	(globalThis as any).window = undefined;
	// Borrar y no asignar `undefined`: en `process.env` eso deja la cadena
	// "undefined", que es verdadera, y el resto de los archivos correría en
	// modo desarrollo sin haberlo pedido.
	delete process.env.DEV;
});

describe('el logger no se llama a sí mismo', () => {
	test('un envío que siempre falla se intenta una sola vez por log', async () => {
		attempts = 0;
		throughOriginalConsole.length = 0;
		deliver = () => Promise.reject(new Error('backend caído'));

		console.warn('el backend no está');
		await settle();

		expect(attempts).toBe(1);
	});

	test('el aviso del fallo sale por la consola original', async () => {
		attempts = 0;
		throughOriginalConsole.length = 0;
		deliver = () => Promise.reject(new Error('backend caído'));

		console.error('el backend no está');
		await settle();

		expect(throughOriginalConsole.some((line) => line.includes('[Logger]'))).toBe(true);
		expect(attempts).toBe(1);
	});

	test('lo que el envío escribe en consola no se reenvía', async () => {
		attempts = 0;
		throughOriginalConsole.length = 0;
		deliver = () => {
			// Un plugin que envuelve `invoke` y se queja: sin la bandera, esto
			// es otro `sendLog`, que vuelve a quejarse.
			console.error('el invoke se quejó');
			console.warn('y avisó');
			return Promise.resolve();
		};

		console.warn('el log que lo dispara');
		await settle();

		expect(attempts).toBe(1);
	});

	test('un envío que falla adentro del `invoke` tampoco se realimenta', async () => {
		attempts = 0;
		throughOriginalConsole.length = 0;
		deliver = () => {
			console.error('el invoke se quejó');
			return Promise.reject(new Error('backend caído'));
		};

		console.error('el log que lo dispara');
		await settle();

		expect(attempts).toBe(1);
	});

	test('los lectores del log avisan sin pasar por la consola reemplazada', async () => {
		attempts = 0;
		throughOriginalConsole.length = 0;
		deliver = () => Promise.reject(new Error('backend caído'));

		expect(await logger.getLogFilePath()).toBe('');
		expect(await logger.readLogFile()).toBe('');
		expect(await logger.getLastLogLines()).toEqual([]);
		await settle();

		// Uno por lector, y ninguno se realimenta.
		expect(attempts).toBe(3);
	});

	test('los logs legítimos que llegan mientras uno viaja no se pierden', async () => {
		attempts = 0;
		throughOriginalConsole.length = 0;
		let release: (() => void) | undefined;
		deliver = () => new Promise<void>((resolve) => (release = resolve));

		console.warn('el primero');
		console.warn('el segundo mientras el primero viaja');
		release?.();
		await settle();

		expect(attempts).toBe(2);
	});
});
