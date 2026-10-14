/**
 * El tablero de fecha se entera solo de lo nuevo (vasak-desktop#237).
 *
 * `tauri-plugin-vsk-store` avisa (`CHANGED_EVENT`) cuando el sincronizador
 * guardó algo del calendario. El tablero, abierto, tiene que volver a pedir el mes que
 * muestra —y el lugar de los eventos del día, que pudo cambiar— sin cerrarlo
 * y volver a abrirlo. Se monta la vista de verdad con el servicio doblado.
 */
import { afterEach, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { mount, type VueWrapper } from '@vue/test-utils';
import { nextTick } from 'vue';
import type { CalendarOccurrence } from '../src/services/calendar.service';
import { board, CALENDAR_CHANGED, sharedEvents } from './support/date-board-doubles';
import { loadComponent, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'date-board-doubles.ts');
const dom = useDom();

const settle = async () => {
	for (let i = 0; i < 8; i++) {
		await nextTick();
		await Promise.resolve();
	}
};

/** Un evento mañana al mediodía: el día que muestra el tablero al abrir. */
function tomorrow(id: string): CalendarOccurrence {
	const now = new Date();
	const start = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1, 12);
	const end = new Date(start.getTime() + 3_600_000);
	return {
		event_id: id,
		occurrence_id: '',
		calendar_id: 'c',
		title: id,
		start: start.toISOString(),
		end: end.toISOString(),
		all_day: false,
		floating: false,
		color: null,
		calendar: null,
	};
}

describe('el tablero de fecha, abierto', () => {
	let Board: any;
	let view: VueWrapper | null = null;
	beforeAll(async () => {
		Board = await loadComponent(dom.workdir(), 'src/views/applets/DateBoardAppletView.vue', DOUBLES, 'DateBoard');
	}, 60_000);
	beforeEach(() => {
		board.reset();
		sharedEvents.clear();
	});
	afterEach(() => {
		view?.unmount();
		view = null;
	});

	const open = async () => {
		view = mount(Board, { attachTo: document.body });
		await settle();
		return view;
	};

	test('escucha el aviso de que cambió el calendario', async () => {
		await open();
		expect(sharedEvents.has(CALENDAR_CHANGED)).toBe(true);
	});

	test('con el aviso vuelve a pedir el mismo mes, sin cerrarse', async () => {
		await open();
		expect(board.calls).toHaveLength(1);
		const shown = board.calls[0];

		board.answer = { state: 'ready', truncated: false, entries: [tomorrow('nuevo')] };
		sharedEvents.get(CALENDAR_CHANGED)?.();
		await settle();

		expect(board.calls).toHaveLength(2);
		expect(board.calls[1]).toEqual(shown);
		// Y lo nuevo se ve.
		expect(view?.text()).toContain('nuevo');
	});

	test('con el aviso vuelve a pedir el lugar de los eventos del día, aunque ya lo tuviera', async () => {
		board.answer = { state: 'ready', truncated: false, entries: [tomorrow('reunion')] };
		await open();
		expect(board.locationCalls).toEqual([['reunion']]);

		// El evento se mudó de sala: el lugar guardado ya no vale.
		board.location = 'Sala 7';
		sharedEvents.get(CALENDAR_CHANGED)?.();
		await settle();

		expect(board.locationCalls).toEqual([['reunion'], ['reunion']]);
		expect(view?.text()).toContain('Sala 7');
		expect(view?.text()).not.toContain('Sala 3');
	});

	test('una respuesta de lugares de antes del aviso no pisa la de después', async () => {
		board.answer = { state: 'ready', truncated: false, entries: [tomorrow('reunion')] };
		let release: () => void = () => {};
		board.locationGate = new Promise((resolve) => {
			release = resolve;
		});
		await open();

		// El evento se mudó mientras el primer pedido seguía en camino.
		board.location = 'Sala 7';
		sharedEvents.get(CALENDAR_CHANGED)?.();
		await settle();
		release();
		await settle();

		expect(view?.text()).toContain('Sala 7');
		expect(view?.text()).not.toContain('Sala 3');
	});
});

describe('el canal del aviso', () => {
	// El aviso ya no lo emite el escritorio: lo emite el plugin del almacén, y
	// sólo para las áreas con que se lo registró. Escuchar otro nombre, o
	// registrarlo sin el calendario, deja el tablero sin enterarse de nada.
	test('el servicio escucha el aviso del plugin del almacén', async () => {
		const service = await import('../src/services/calendar.service');
		const { CHANGED_EVENT } = await import('@vasakgroup/plugin-vsk-store');
		expect(service.CALENDAR_CHANGED).toBe(CHANGED_EVENT);
	});

	test('el escritorio registra el plugin del almacén para el calendario', async () => {
		const lib = await Bun.file(join(import.meta.dir, '..', 'src-tauri/src/lib.rs')).text();
		expect(lib).toMatch(/\.plugin\(tauri_plugin_vsk_store::init\(&\[[^\]]*"calendar"[^\]]*\]\)\)/);
	});
});
