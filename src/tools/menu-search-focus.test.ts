/**
 * El menú tiene que abrirse listo para escribir.
 *
 * La ventana del menú se esconde en vez de destruirse, así que la vista se
 * monta **una sola vez en la vida del proceso**. Todo lo que dependa de un
 * montaje corre esa única vez: el `autofocus` del campo enfocaba la primera
 * apertura y ninguna más, y lo que cubría las demás era un
 * `document.getElementById('search')?.focus()` sobre un id que dejó de existir
 * cuando el campo pasó a ser el `SearchField` de la librería. El `?.` se tragaba
 * el `null`, así que el menú abría sin foco sin que nada fallara.
 *
 * Por eso la prueba que importa es la **segunda** apertura, y por eso además se
 * comprueba el cable en la vista: el módulo puede estar perfecto y no llamarlo
 * nadie, que es exactamente lo que pasaba.
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
	DEFAULT_RETRIES,
	type FocusableSearchField,
	focusMenuSearch,
	prepareMenuSearch,
} from '@/tools/menu-search-focus';

/**
 * Un campo de juguete que cuenta los intentos y decide cuándo acepta el foco.
 *
 * `enfocar()` devuelve si el foco llegó, que es lo que distingue «pedí el foco»
 * de «el campo lo tiene».
 */
function fieldThatAcceptsAfter(attemptsToRefuse: number) {
	let attempts = 0;
	const field: FocusableSearchField & { attempts(): number } = {
		enfocar() {
			attempts += 1;
			return attempts > attemptsToRefuse;
		},
		attempts: () => attempts,
	};
	return field;
}

/** Un `setTimeout` que corre en el acto, para no esperar los reintentos. */
function immediateSchedule() {
	let scheduled = 0;
	return {
		run: (fn: () => void, _ms: number) => {
			scheduled += 1;
			fn();
			return () => {};
		},
		scheduled: () => scheduled,
	};
}

/**
 * Un `setTimeout` que guarda lo pendiente en vez de correrlo, para poder mirar
 * qué pasa cuando llega tarde.
 */
function pendingSchedule() {
	let queued: (() => void) | null = null;
	let cancels = 0;
	return {
		run: (fn: () => void, _ms: number) => {
			queued = fn;
			return () => {
				cancels += 1;
				queued = null;
			};
		},
		/** Dispara lo que haya quedado en cola, como haría el temporizador. */
		fire: () => {
			const pending = queued;
			queued = null;
			pending?.();
		},
		cancels: () => cancels,
		hasPending: () => queued !== null,
	};
}

describe('prepareMenuSearch', () => {
	test('enfoca el campo al abrir', () => {
		const field = fieldThatAcceptsAfter(0);
		prepareMenuSearch({ field: () => field, clear: () => {} });

		expect(field.attempts()).toBe(1);
	});

	test('la segunda apertura enfoca igual que la primera', () => {
		// El bug: la vista no se vuelve a montar, así que sólo la primera abría
		// con foco. Se abre dos veces con el mismo campo, sin montar nada en el
		// medio.
		const field = fieldThatAcceptsAfter(0);
		const abrir = () => prepareMenuSearch({ field: () => field, clear: () => {} });

		abrir();
		abrir();

		expect(field.attempts()).toBe(2);
	});

	test('el filtro llega vacío a cada apertura', () => {
		let filter = 'firefox';
		prepareMenuSearch({
			field: () => fieldThatAcceptsAfter(0),
			clear: () => {
				filter = '';
			},
		});

		expect(filter).toBe('');
	});

	test('vacía el filtro aunque el campo todavía no esté montado', () => {
		// El evento de foco de la ventana puede llegar antes que el montaje.
		let cleared = false;
		expect(() =>
			prepareMenuSearch({
				field: () => null,
				clear: () => {
					cleared = true;
				},
			})
		).not.toThrow();
		expect(cleared).toBe(true);
	});

	test('insiste mientras el foco no llegue', () => {
		// El WebView puede no tener el foco del documento en el instante en que
		// el compositor se lo da a la ventana.
		const field = fieldThatAcceptsAfter(2);
		const schedule = immediateSchedule();

		prepareMenuSearch({
			field: () => field,
			clear: () => {},
			schedule: schedule.run,
		});

		expect(field.attempts()).toBe(3);
		expect(schedule.scheduled()).toBe(2);
	});

	test('deja de insistir en cuanto el foco llega', () => {
		const field = fieldThatAcceptsAfter(1);
		const schedule = immediateSchedule();

		prepareMenuSearch({
			field: () => field,
			clear: () => {},
			schedule: schedule.run,
		});

		expect(field.attempts()).toBe(2);
		expect(schedule.scheduled()).toBe(1);
	});

	test('no insiste para siempre con un campo que nunca acepta', () => {
		// El menú vacío desactiva el campo, y un campo desactivado no toma el
		// foco: sin tope quedaría un temporizador rebotando mientras el menú
		// esté abierto.
		const field = fieldThatAcceptsAfter(Number.POSITIVE_INFINITY);
		const schedule = immediateSchedule();

		prepareMenuSearch({
			field: () => field,
			clear: () => {},
			schedule: schedule.run,
		});

		expect(field.attempts()).toBe(DEFAULT_RETRIES + 1);
		expect(schedule.scheduled()).toBe(DEFAULT_RETRIES);
	});
});

describe('cortar un intento en curso', () => {
	test('un reintento cancelado no le roba el foco a nadie', () => {
		// El menú se cierra mientras quedan reintentos vivos: el que llegue
		// tarde enfocaría un campo que ya no está a la vista.
		const field = fieldThatAcceptsAfter(Number.POSITIVE_INFINITY);
		const schedule = pendingSchedule();

		const attempt = prepareMenuSearch({
			field: () => field,
			clear: () => {},
			schedule: schedule.run,
		});
		const beforeCancel = field.attempts();
		attempt.cancel();
		schedule.fire();

		expect(field.attempts()).toBe(beforeCancel);
	});

	test('cancelar apaga el temporizador y no sólo el efecto', () => {
		const schedule = pendingSchedule();

		const attempt = focusMenuSearch({
			field: () => fieldThatAcceptsAfter(Number.POSITIVE_INFINITY),
			schedule: schedule.run,
		});
		expect(schedule.hasPending()).toBe(true);
		attempt.cancel();

		expect(schedule.cancels()).toBe(1);
		expect(schedule.hasPending()).toBe(false);
	});

	test('cancelar dos veces no explota', () => {
		const attempt = focusMenuSearch({ field: () => fieldThatAcceptsAfter(0) });

		expect(() => {
			attempt.cancel();
			attempt.cancel();
		}).not.toThrow();
	});
});

describe('focusMenuSearch', () => {
	test('enfoca sin vaciar nada', () => {
		// Es lo que se llama cuando el campo se habilita con el menú ya abierto:
		// para entonces, lo que hay escrito lo escribió el usuario.
		const field = fieldThatAcceptsAfter(0);
		focusMenuSearch({ field: () => field });

		expect(field.attempts()).toBe(1);
	});

	test('un intento nuevo alcanza cuando el campo se habilita', () => {
		// `isMenuEmpty` arranca en verdadero y desactiva el campo, y un campo
		// desactivado no toma el foco. Si la carga del menú tarda más que los
		// reintentos, se agotan contra un campo que no puede tomarlo.
		let enabled = false;
		let focusedTimes = 0;
		const field: FocusableSearchField = {
			enfocar: () => {
				if (!enabled) return false;
				focusedTimes += 1;
				return true;
			},
		};
		const schedule = immediateSchedule();

		focusMenuSearch({ field: () => field, schedule: schedule.run });

		expect(focusedTimes).toBe(0);
		expect(schedule.scheduled()).toBe(DEFAULT_RETRIES);

		// Por eso hace falta volver a intentarlo cuando el campo se habilita, y
		// no dar la apertura por perdida.
		enabled = true;
		focusMenuSearch({ field: () => field });

		expect(focusedTimes).toBe(1);
	});
});

describe('el cable en la vista', () => {
	const source = readFileSync(join(import.meta.dir, '..', 'views', 'MenuView.vue'), 'utf8');

	test('el menú prepara la búsqueda al ganar el foco la ventana', () => {
		// Y no en `onMounted`, que corre una sola vez porque la ventana se
		// esconde en vez de destruirse.
		const handler = source.slice(source.indexOf('onFocusChanged'));

		expect(handler).toMatch(/prepareMenuSearch\(/);
	});

	test('el campo lleva el `ref` que permite enfocarlo', () => {
		expect(source).toMatch(/<SearchField\s[^>]*ref="searchField"/);
	});

	test('el campo de la librería sigue exponiendo `enfocar`', () => {
		// Todo el arreglo se apoya en esto. Si una versión de `vue-libvasak`
		// renombra o saca el método, el `ref` queda apuntando a algo que no
		// enfoca y el menú vuelve a abrir mudo — sin que nada falle, porque el
		// `?.` se lo traga igual que se tragaba el id.
		const contrato = readFileSync(
			join(
				import.meta.dir,
				'..',
				'..',
				'node_modules',
				'@vasakgroup',
				'vue-libvasak',
				'dist',
				'types',
				'search',
				'SearchField.vue.d.ts'
			),
			'utf8'
		);

		expect(contrato).toMatch(/enfocar:\s*typeof\s+enfocar/);
		expect(contrato).toMatch(/declare\s+function\s+enfocar\(\):\s*boolean/);
	});

	test('el menú reenfoca cuando el campo deja de estar desactivado', () => {
		// Y sin vaciar el filtro: para cuando el menú termina de cargar, lo que
		// hay escrito lo escribió el usuario.
		const observador = source.slice(source.indexOf('watch(\n\tisMenuEmpty'));

		expect(observador).toMatch(/focusMenuSearch\(/);
		expect(observador.slice(0, observador.indexOf('\n);'))).not.toMatch(/clear:/);
	});

	test('los reintentos se cortan al cerrar y al desmontar', () => {
		// Quedan hasta 150 ms de reintentos vivos: uno que llegue tarde le
		// robaría el foco a donde el usuario ya esté.
		const alCerrar = source.slice(
			source.indexOf('const closeAfterAnimation'),
			source.indexOf('const appsOfCategory')
		);
		const alDesmontar = source.slice(
			source.indexOf('onBeforeUnmount('),
			source.indexOf('watch(\n\tisMenuEmpty')
		);

		expect(alCerrar).toMatch(/searchFocus\?\.cancel\(\)/);
		expect(alDesmontar).toMatch(/searchFocus\?\.cancel\(\)/);
	});

	test('el campo no se vuelve a buscar por `id` en el documento', () => {
		// Buscarlo por id no fallaba: devolvía `null` y el `?.` se lo tragaba.
		// Por eso sobrevivió intacto a la migración al campo de la librería, y
		// por eso la guardia es sobre la forma entera y no sobre aquel id.
		expect(source).not.toMatch(/getElementById/);
	});
});
