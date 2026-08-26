import { describe, expect, test } from 'bun:test';
import {
	crearEscuchaDeVisibilidad,
	type DocumentoObservable,
} from '@/tools/composables/escucha-de-visibilidad';

/** Un `document` de juguete que cuenta lo que se le engancha y se le saca. */
function documentoFalso() {
	const manejadores = new Set<() => void>();
	let enganches = 0;
	let desenganches = 0;
	const doc: DocumentoObservable & {
		disparar(): void;
		enganches(): number;
		desenganches(): number;
		vivos(): number;
	} = {
		hidden: false,
		addEventListener(_tipo, manejador) {
			manejadores.add(manejador);
			enganches += 1;
		},
		removeEventListener(_tipo, manejador) {
			manejadores.delete(manejador);
			desenganches += 1;
		},
		disparar() {
			for (const m of [...manejadores]) m();
		},
		enganches: () => enganches,
		desenganches: () => desenganches,
		vivos: () => manejadores.size,
	};
	return doc;
}

describe('crearEscuchaDeVisibilidad', () => {
	test('soltar deja el documento sin escuchas', () => {
		// El bug: al desmontarse el último consumidor el escucha quedaba vivo, y
		// esconder y mostrar la ventana seguía disparando trabajo sin nadie
		// mirando.
		const doc = documentoFalso();
		let veces = 0;
		const escucha = crearEscuchaDeVisibilidad(doc, () => {
			veces += 1;
		});

		escucha.enganchar();
		doc.disparar();
		expect(veces).toBe(1);

		escucha.soltar();
		expect(doc.vivos()).toBe(0);
		doc.disparar();
		expect(veces).toBe(1);
	});

	test('enganchar dos veces no duplica el escucha', () => {
		// Con dos enganchados, cada vuelta de la ventana haría el trabajo dos
		// veces, y soltar una sola dejaría el otro vivo.
		const doc = documentoFalso();
		const escucha = crearEscuchaDeVisibilidad(doc, () => {});

		escucha.enganchar();
		escucha.enganchar();
		expect(doc.enganches()).toBe(1);

		escucha.soltar();
		expect(doc.vivos()).toBe(0);
	});

	test('soltar sin haber enganchado no toca el documento', () => {
		const doc = documentoFalso();
		crearEscuchaDeVisibilidad(doc, () => {}).soltar();
		expect(doc.desenganches()).toBe(0);
	});

	test('sólo avisa cuando la ventana está a la vista', () => {
		// Al esconderse no hay que refrescar nada: es justo lo que la pausa evita.
		const doc = documentoFalso();
		let veces = 0;
		const escucha = crearEscuchaDeVisibilidad(doc, () => {
			veces += 1;
		});
		escucha.enganchar();

		doc.hidden = true;
		doc.disparar();
		expect(veces).toBe(0);

		doc.hidden = false;
		doc.disparar();
		expect(veces).toBe(1);
	});

	test('sin document no explota ni queda enganchado', () => {
		// Al arrancar una ventana de Tauri no siempre hay uno.
		const escucha = crearEscuchaDeVisibilidad(undefined, () => {});
		escucha.enganchar();
		expect(escucha.enganchado()).toBe(false);
		escucha.soltar();
	});
});
