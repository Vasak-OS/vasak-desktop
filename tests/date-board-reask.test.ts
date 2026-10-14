/**
 * El botón «pedir permiso de nuevo» del tablero de fecha (ola 2 de «pedir
 * permiso de nuevo», pedido del 13/10/2026).
 *
 * Cuando el escritorio no tiene permiso para leer el calendario guardado, el
 * tablero muestra el vacío «sin permiso» con un botón que le pide a
 * `vasak-permissions` que vuelva a preguntar (`RequestAgain`). Si el servicio
 * olvida la decisión anterior, el tablero relee y esa lectura reabre el
 * diálogo; un re-pedido demasiado seguido o un fallo quedan dichos sin sacar el
 * estado «sin permiso». Se monta la vista de verdad con el servicio doblado.
 */
import { afterEach, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount, type VueWrapper } from '@vue/test-utils';
import { nextTick } from 'vue';
import type { CalendarOccurrence } from '../src/services/calendar.service';
import { board, sharedEvents } from './support/date-board-doubles';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'date-board-doubles.ts');
const dom = useDom();
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');
const ES = Bun.YAML.parse(read('src-tauri/locales/es.yml')) as Record<string, any>;
const EN = Bun.YAML.parse(read('src-tauri/locales/en.yml')) as Record<string, any>;

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
	return {
		event_id: id,
		occurrence_id: '',
		calendar_id: 'c',
		title: id,
		start: start.toISOString(),
		end: new Date(start.getTime() + 3_600_000).toISOString(),
		all_day: false,
		floating: false,
		color: null,
		calendar: null,
	};
}

describe('el tablero de fecha sin permiso', () => {
	let Board: any;
	let view: VueWrapper | null = null;
	beforeAll(async () => {
		Board = await loadComponent(dom.workdir(), 'src/views/applets/DateBoardAppletView.vue', DOUBLES, 'DateBoard');
	}, 60_000);
	beforeEach(() => {
		board.reset();
		sharedEvents.clear();
		board.answer = { state: 'denied' };
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

	test('muestra el botón para volver a pedir el permiso', async () => {
		await open();
		const button = view?.find('[data-reask-permission]');
		expect(button?.exists()).toBe(true);
		expect(view?.text()).toContain('views.dateBoard.reask');
	});

	test('al tocarlo, si el servicio olvida la decisión, relee y aparecen los eventos', async () => {
		await open();
		expect(board.calls).toHaveLength(1);

		// El servicio reabre el diálogo; la relectura ya trae los eventos.
		board.reaskOutcome = { outcome: 'reopened' };
		board.answer = { state: 'ready', truncated: false, entries: [tomorrow('reunion')] };
		await view?.find('[data-reask-permission]').trigger('click');
		await settle();

		expect(board.reaskCalls).toBe(1);
		// Volvió a leer el calendario: esa lectura es la que abre el diálogo.
		expect(board.calls).toHaveLength(2);
		expect(view?.text()).toContain('reunion');
		expect(view?.find('[data-reask-permission]').exists()).toBe(false);
	});

	test('un re-pedido demasiado seguido lo dice y no relee', async () => {
		await open();
		expect(board.calls).toHaveLength(1);

		board.reaskOutcome = { outcome: 'too_soon' };
		await view?.find('[data-reask-permission]').trigger('click');
		await settle();

		expect(board.reaskCalls).toBe(1);
		// No reabrió nada: no se relee, y el estado «sin permiso» sigue.
		expect(board.calls).toHaveLength(1);
		const note = view?.find('[data-reask-note]');
		expect(note?.exists()).toBe(true);
		expect(note?.text()).toContain('views.dateBoard.reaskTooSoon');
		expect(view?.find('[data-reask-permission]').exists()).toBe(true);
	});

	test('un fallo lo dice y no relee', async () => {
		await open();
		board.reaskOutcome = { outcome: 'failed', detail: 'x' };
		await view?.find('[data-reask-permission]').trigger('click');
		await settle();

		expect(board.calls).toHaveLength(1);
		expect(view?.find('[data-reask-note]').text()).toContain('views.dateBoard.reaskFailed');
	});
});

describe('los textos del botón', () => {
	test('las tres claves están en los dos catálogos', () => {
		for (const catalog of [ES, EN]) {
			expect(typeof catalog.views.dateBoard.reask).toBe('string');
			expect(typeof catalog.views.dateBoard.reaskTooSoon).toBe('string');
			expect(typeof catalog.views.dateBoard.reaskFailed).toBe('string');
		}
	});
});

describe('el comando del re-pedido', () => {
	test('el servicio y el backend nombran el mismo comando', async () => {
		const service = read('src/services/calendar.service.ts');
		const lib = read('src-tauri/src/lib.rs');
		const command = read('src-tauri/src/commands/store_access.rs');
		expect(service).toContain("invoke<ReaskOutcome>('request_calendar_access_again')");
		expect(lib).toContain('request_calendar_access_again');
		expect(command).toContain('pub async fn request_calendar_access_again()');
		// Va al recurso del calendario guardado, en el bus de sistema.
		expect(command).toContain('"store.calendar"');
		expect(command).toContain('Connection::system()');
	});
});
