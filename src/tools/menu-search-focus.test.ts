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
		},
		scheduled: () => scheduled,
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

	test('el campo no se vuelve a buscar por `id` en el documento', () => {
		// Buscarlo por id no fallaba: devolvía `null` y el `?.` se lo tragaba.
		// Por eso sobrevivió intacto a la migración al campo de la librería, y
		// por eso la guardia es sobre la forma entera y no sobre aquel id.
		expect(source).not.toMatch(/getElementById/);
	});
});
