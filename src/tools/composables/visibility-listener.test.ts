import { describe, expect, test } from 'bun:test';
import {
	createVisibilityListener,
	type ObservableDocument,
} from '@/tools/composables/visibility-listener';

/** Un `document` de juguete que cuenta lo que se le engancha y se le saca. */
function fakeDocument() {
	const listeners = new Set<() => void>();
	let attachCount = 0;
	let detachCount = 0;
	const doc: ObservableDocument & {
		fire(): void;
		attaches(): number;
		detaches(): number;
		alive(): number;
	} = {
		hidden: false,
		addEventListener(_type, listener) {
			listeners.add(listener);
			attachCount += 1;
		},
		removeEventListener(_type, listener) {
			listeners.delete(listener);
			detachCount += 1;
		},
		fire() {
			for (const handler of [...listeners]) handler();
		},
		attaches: () => attachCount,
		detaches: () => detachCount,
		alive: () => listeners.size,
	};
	return doc;
}

describe('createVisibilityListener', () => {
	test('soltar deja el documento sin escuchas', () => {
		// El bug: al desmontarse el último consumidor el escucha quedaba vivo, y
		// esconder y mostrar la ventana seguía disparando trabajo sin nadie
		// mirando.
		const doc = fakeDocument();
		let calls = 0;
		const visibility = createVisibilityListener(doc, () => {
			calls += 1;
		});

		visibility.attach();
		doc.fire();
		expect(calls).toBe(1);

		visibility.detach();
		expect(doc.alive()).toBe(0);
		doc.fire();
		expect(calls).toBe(1);
	});

	test('enganchar dos veces no duplica el escucha', () => {
		// Con dos enganchados, cada vuelta de la ventana haría el trabajo dos
		// veces, y soltar una sola dejaría el otro vivo.
		const doc = fakeDocument();
		const visibility = createVisibilityListener(doc, () => {});

		visibility.attach();
		visibility.attach();
		expect(doc.attaches()).toBe(1);

		visibility.detach();
		expect(doc.alive()).toBe(0);
	});

	test('soltar sin haber enganchado no toca el documento', () => {
		const doc = fakeDocument();
		createVisibilityListener(doc, () => {}).detach();
		expect(doc.detaches()).toBe(0);
	});

	test('sólo avisa cuando la ventana está a la vista', () => {
		// Al esconderse no hay que refrescar nada: es justo lo que la pausa evita.
		const doc = fakeDocument();
		let calls = 0;
		const visibility = createVisibilityListener(doc, () => {
			calls += 1;
		});
		visibility.attach();

		doc.hidden = true;
		doc.fire();
		expect(calls).toBe(0);

		doc.hidden = false;
		doc.fire();
		expect(calls).toBe(1);
	});

	test('sin document no explota ni queda enganchado', () => {
		// Al arrancar una ventana de Tauri no siempre hay uno.
		const visibility = createVisibilityListener(undefined, () => {});
		visibility.attach();
		expect(visibility.attached()).toBe(false);
		visibility.detach();
	});
});
