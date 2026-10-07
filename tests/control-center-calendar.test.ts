/**
 * El calendario del mes en el centro de control (vasak-desktop#183, ancla #174).
 *
 * Tocar la fecha de la tarjeta de usuario abre el mes dentro del centro, como
 * la ficha de un mosaico, y «Volver» deja lo que había. Lo puro se prueba
 * directo; la ficha y la tarjeta, montadas con el servicio de eventos doblado;
 * la vista, montada con sus componentes doblados.
 */
import { afterEach, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { addMonths, monthOf, toIsoDate } from '@vasakgroup/vue-libvasak';
import { mount, type VueWrapper } from '@vue/test-utils';
import { nextTick } from 'vue';
import { compileScript, parse } from 'vue/compiler-sfc';
import type { CalendarOccurrence } from '../src/services/calendar.service';
import { localeTag, marksFromReply, weekStartFor } from '../src/tools/calendar-sheet';
import { calendar, sharedEvents } from './support/calendar-doubles';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'calendar-doubles.ts');
const dom = useDom();
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

const settle = async () => {
	for (let i = 0; i < 8; i++) {
		await nextTick();
		await Promise.resolve();
	}
};

/** Una vez de evento como la manda el almacén. */
function occurrence(id: string, start: string, end: string, allDay = false): CalendarOccurrence {
	return {
		event_id: id,
		occurrence_id: '',
		calendar_id: 'c',
		title: id,
		start,
		end,
		all_day: allDay,
		floating: false,
		color: null,
		calendar: null,
	};
}

/** Un día de este mes a las 12 locales, en RFC 3339: cae en ese día en cualquier zona. */
function noonOf(day: number): { start: string; end: string; iso: string } {
	const now = new Date();
	const start = new Date(now.getFullYear(), now.getMonth(), day, 12);
	const end = new Date(now.getFullYear(), now.getMonth(), day, 13);
	return { start: start.toISOString(), end: end.toISOString(), iso: toIsoDate(start) };
}

describe('las cuentas', () => {
	test('el idioma de la interfaz como etiqueta de Intl', () => {
		expect(localeTag('es')).toBe('es');
		expect(localeTag('en-US')).toBe('en-US');
		expect(localeTag('es_AR.UTF-8')).toBe('es-AR');
		expect(localeTag('')).toBeUndefined();
		expect(localeTag(undefined)).toBeUndefined();
		expect(localeTag('no es un idioma!')).toBeUndefined();
	});

	test('la semana empieza según el idioma', () => {
		expect(weekStartFor('es')).toBe(1);
		expect(weekStartFor('es_AR.UTF-8')).toBe(1);
		expect(weekStartFor('en-US')).toBe(0);
		expect(weekStartFor('en')).toBe(0);
	});

	test('sólo una respuesta con eventos marca días', () => {
		const first = noonOf(3);
		const second = noonOf(17);
		expect(
			marksFromReply({
				state: 'ready',
				truncated: false,
				entries: [occurrence('a', second.start, second.end), occurrence('b', first.start, first.end)],
			})
		).toEqual([first.iso, second.iso]);
		expect(marksFromReply({ state: 'unavailable' })).toEqual([]);
		expect(marksFromReply({ state: 'denied' })).toEqual([]);
		expect(marksFromReply({ state: 'failed', detail: 'x' })).toEqual([]);
		expect(marksFromReply(null)).toEqual([]);
	});

	test('un día completo cae en su día, como en el tablero de fecha', () => {
		// El almacén lo manda a medianoche UTC; leído en local caía un día antes.
		expect(
			marksFromReply({
				state: 'ready',
				truncated: false,
				entries: [occurrence('f', '2026-10-23T00:00:00Z', '2026-10-24T00:00:00Z', true)],
			})
		).toEqual(['2026-10-23']);
	});
});

describe('la ficha del calendario, montada', () => {
	let Sheet: any;
	let view: VueWrapper | null = null;
	beforeAll(async () => {
		Sheet = await loadComponent(dom.workdir(), 'src/components/areas/control-center/CalendarSheet.vue', DOUBLES, 'CalendarSheet');
	}, 60_000);
	beforeEach(() => calendar.reset());
	afterEach(() => {
		view?.unmount();
		view = null;
	});

	const open = async () => {
		view = mount(Sheet, { attachTo: document.body });
		await settle();
		return view;
	};

	test('abre en el mes de hoy, con hoy marcado', async () => {
		const sheet = await open();
		const today = toIsoDate(new Date());
		const cell = sheet.find(`[data-date="${today}"]`);
		expect(cell.exists()).toBe(true);
		expect(cell.attributes('data-today')).toBe('true');
		expect(cell.attributes('aria-current')).toBe('date');
		expect(sheet.findAll('[data-today]')).toHaveLength(1);
		// El título es el mes de hoy, en el idioma.
		const title = new Date().toLocaleDateString('es', { month: 'long', year: 'numeric' });
		expect(sheet.find('[data-month-title]').text().toLowerCase()).toBe(title.toLowerCase());
		// Las casillas del mes son las del mes de hoy.
		const inMonth = sheet.findAll('[data-in-month="true"]').map((day) => day.attributes('data-date') ?? '');
		expect(inMonth.every((date) => monthOf(date) === monthOf(today))).toBe(true);
	});

	test('la semana empieza en lunes en castellano', async () => {
		const sheet = await open();
		const first = sheet.find('[role="row"] [role="columnheader"]');
		expect(first.attributes('aria-label')?.toLowerCase()).toBe('lunes');
	});

	test('el foco va a «Volver», y «Volver» avisa', async () => {
		const sheet = await open();
		const back = sheet.find('[data-calendar-back] button, button[data-calendar-back]');
		expect(back.exists()).toBe(true);
		expect(document.activeElement === back.element).toBe(true);
		await back.trigger('click');
		expect(sheet.emitted('back')).toHaveLength(1);
	});

	test('pide los eventos al abrir, una sola vez, por las seis semanas que se ven', async () => {
		await open();
		expect(calendar.calls).toHaveLength(1);
		const cells = view?.findAll('[data-date]') ?? [];
		const first = cells[0]?.attributes('data-date') ?? '';
		const call = calendar.calls[0];
		expect(call && toIsoDate(new Date(call.from))).toBe(first);
	});

	test('marca los días con eventos que contesta el servicio', async () => {
		const third = noonOf(3);
		const twentieth = noonOf(20);
		calendar.answer = {
			state: 'ready',
			truncated: false,
			entries: [occurrence('a', third.start, third.end), occurrence('b', twentieth.start, twentieth.end)],
		};
		const sheet = await open();
		const marked = sheet.findAll('[data-marked]').map((day) => day.attributes('data-date'));
		expect(marked).toEqual([third.iso, twentieth.iso]);
	});

	test('navega de mes y vuelve a pedir sólo entonces', async () => {
		const sheet = await open();
		const thisMonth = monthOf(toIsoDate(new Date()));
		const buttons = sheet.findAll('[data-month-calendar] button').filter((b) => !b.attributes('data-date'));
		const [previous, next] = buttons;
		await next?.trigger('click');
		await settle();
		const nextMonth = addMonths(thisMonth, 1);
		expect(sheet.findAll('[data-in-month="true"]')[0]?.attributes('data-date')).toBe(`${nextMonth}-01`);
		expect(sheet.findAll('[data-today]')).toHaveLength(0);
		expect(calendar.calls).toHaveLength(2);

		await previous?.trigger('click');
		await previous?.trigger('click');
		await settle();
		expect(sheet.findAll('[data-in-month="true"]')[0]?.attributes('data-date')).toBe(`${addMonths(thisMonth, -1)}-01`);
		expect(calendar.calls).toHaveLength(4);
	});

	test('una respuesta vieja que llega tarde no pisa las marcas del mes nuevo', async () => {
		// Un día que se ve en la cuadrícula del mes siguiente: si la respuesta
		// vieja pisara, el punto aparecería.
		const now = new Date();
		const nextTenth = new Date(now.getFullYear(), now.getMonth() + 1, 10, 12);
		const day = { start: nextTenth.toISOString(), end: new Date(nextTenth.getTime() + 3_600_000).toISOString() };
		let release: (value: import('../src/services/calendar.service').CalendarReply) => void = () => {};
		calendar.answer = () =>
			new Promise((resolve) => {
				release = resolve;
			});
		const sheet = await open();
		const stale = release;
		calendar.answer = { state: 'ready', truncated: false, entries: [] };
		const next = sheet.findAll('[data-month-calendar] button').filter((b) => !b.attributes('data-date'))[1];
		await next?.trigger('click');
		await settle();
		stale({ state: 'ready', truncated: false, entries: [occurrence('viejo', day.start, day.end)] });
		await settle();
		expect(sheet.findAll('[data-marked]')).toHaveLength(0);
	});

	test('sin servicio, sin permiso o con error: el mes, sin marcas y sin romperse', async () => {
		for (const answer of [
			{ state: 'unavailable' } as const,
			{ state: 'denied' } as const,
			{ state: 'failed', detail: 'boom' } as const,
			new Error('sin bus de sesión'),
		]) {
			calendar.reset();
			calendar.answer = answer;
			const sheet = await open();
			expect(sheet.find(`[data-date="${toIsoDate(new Date())}"]`).exists()).toBe(true);
			expect(sheet.findAll('[data-marked]')).toHaveLength(0);
			view?.unmount();
			view = null;
		}
		// El error se anota, no se tira.
		expect(calendar.errors).toHaveLength(1);
	});

	test('la ficha va sobre la superficie del centro y se acomoda por contenedor', () => {
		const source = read('src/components/areas/control-center/CalendarSheet.vue');
		expect(source).toContain('bg-ui-surface/70');
		expect(source).toContain('@container');
		expect(source).not.toMatch(/backdrop-blur|setInterval|setTimeout/);
	});
});

describe('la fecha de la tarjeta', () => {
	let Card: any;
	beforeAll(async () => {
		// Quién es: el `invoke` de mentira contesta vacío a todo.
		const internals = (globalThis as unknown as { __TAURI_INTERNALS__: { invoke: (cmd: string, args: unknown) => Promise<unknown> } })
			.__TAURI_INTERNALS__;
		const fallback = internals.invoke;
		internals.invoke = async (cmd, args) =>
			cmd.startsWith('plugin:user-data|') ? { username: 'ana', full_name: 'Ana Pérez', avatar_data: '' } : fallback(cmd, args);
		Card = await loadComponent(dom.workdir(), 'src/components/cards/UserControlCenterCard.vue', DOUBLES, 'UserCard');
	}, 60_000);

	test('es un botón de la librería que abre el calendario', async () => {
		const card = mount(Card, { attachTo: document.body });
		await settle();
		const date = card.find('button[data-user-date]');
		expect(date.exists()).toBe(true);
		expect(date.attributes('aria-expanded')).toBe('false');
		// Dice qué hace, no sólo la fecha: sin catálogo, la clave del texto.
		expect(date.attributes('aria-label')).toStartWith('views.controlCenter.openCalendar');
		await date.trigger('click');
		expect(card.emitted('open-calendar')).toHaveLength(1);
		await card.setProps({ calendarOpen: true });
		expect(card.find('button[data-user-date]').attributes('aria-expanded')).toBe('true');
		card.unmount();
	});
});

/**
 * Compila la vista con cada `@/…` mandado a los dobles.
 *
 * Cada componente del escritorio es el doble de su nombre, o un doble mudo si
 * la prueba no lo conoce: la vista suma controles seguido (la luz nocturna, el
 * micrófono…) y esta prueba no tiene por qué romperse con cada uno. Lo que se
 * importa por nombre sale del mismo módulo de dobles.
 */
async function loadView() {
	const file = join(ROOT, 'src/views/ControlCenterView.vue');
	const { descriptor } = parse(read('src/views/ControlCenterView.vue'), { filename: file });
	const compiled = compileScript(descriptor, { id: 'ControlCenterView', inlineTemplate: true });
	const code = `import * as __doubles from ${JSON.stringify(DOUBLES)};\n${compiled.content
		.replace(/import\s+type\s+\{[^}]*\}\s+from\s+(['"])@\/[^'"]+\1;?/g, '')
		.replace(
			/import\s+(\w+)\s+from\s+(['"])@\/[^'"]+\.vue\2;?/g,
			"const $1 = __doubles.$1 ?? __doubles.silent('$1');"
		)
		.replace(
			/import\s+\{([^}]*)\}\s+from\s+(['"])@\/[^'"]+\2;?/g,
			(_all, names: string) =>
				`const {${names.replace(/\btype\s+\w+\s*,?/g, '').replace(/\s+as\s+/g, ': ')}} = __doubles;`
		)}`;
	const out = join(dom.workdir(), 'ControlCenterView.ts');
	writeFileSync(out, code);
	return (await import(out)).default;
}

describe('el centro, con el calendario', () => {
	let View: any;
	beforeAll(async () => {
		View = await loadView();
	}, 60_000);

	test('tocar la fecha abre el calendario en lugar de lo de abajo, y «Volver» lo deja como estaba', async () => {
		const center = mount(View, { attachTo: document.body });
		await settle();
		const notifications = () => center.find('[data-fake-notifications]').element as HTMLElement;
		expect(center.find('[data-calendar-sheet]').exists()).toBe(false);
		expect(notifications().style.display).not.toBe('none');

		await center.find('[data-user-date]').trigger('click');
		await settle();
		expect(center.find('[data-calendar-sheet]').exists()).toBe(true);
		expect(center.find('[data-user-date]').attributes('aria-expanded')).toBe('true');
		// Las notificaciones siguen montadas, escondidas: no se vuelven a pedir.
		expect(notifications().style.display).toBe('none');

		await center.find('[data-calendar-back]').trigger('click');
		await settle();
		expect(center.find('[data-calendar-sheet]').exists()).toBe(false);
		expect(notifications().style.display).not.toBe('none');
		// El foco vuelve a la fecha que lo abrió.
		// Se compara por identidad: el diff de dos nodos del DOM son millones de líneas.
		expect(document.activeElement === center.find('[data-user-date]').element).toBe(true);
		center.unmount();
	});

	test('al volver a abrir el centro, el calendario está cerrado', async () => {
		const center = mount(View, { attachTo: document.body });
		await settle();
		await center.find('[data-user-date]').trigger('click');
		await settle();
		expect(center.find('[data-calendar-sheet]').exists()).toBe(true);
		sharedEvents.get('window-shown')?.();
		await settle();
		expect(center.find('[data-calendar-sheet]').exists()).toBe(false);
		center.unmount();
	});

	test('los textos nuevos existen en los dos idiomas', () => {
		for (const locale of ['es', 'en']) {
			const catalog = Bun.YAML.parse(read(`src-tauri/locales/${locale}.yml`)) as {
				views: { controlCenter: Record<string, string> };
			};
			expect(catalog.views.controlCenter.calendar, locale).toBeString();
			expect(catalog.views.controlCenter.openCalendar, locale).toContain('{0}');
		}
	});
});
