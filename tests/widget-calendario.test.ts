import { describe, expect, test } from 'bun:test';
import type { CalendarOccurrence } from '../src/services/calendar.service';
import {
	AGENDA_MIN_HEIGHT,
	agendaFromReply,
	showsAgenda,
	todayRange,
} from '../src/tools/calendar-widget';
import { WIDGETS } from '../src/tools/widgets/catalog';

const evento = (id: string, start: string, all_day = false): CalendarOccurrence => ({
	event_id: id,
	occurrence_id: id,
	calendar_id: 'c',
	title: id,
	start,
	end: start,
	all_day,
	floating: false,
	color: null,
	calendar: null,
});

describe('el widget de calendario', () => {
	test('está en el catálogo con un mínimo que deja ver el mes', () => {
		expect(WIDGETS.calendar.min).toEqual({ w: 3, h: 3 });
		expect(WIDGETS.calendar.unique).toBe(true);
	});

	test('la lista de hoy pide cuatro filas', () => {
		expect(showsAgenda(AGENDA_MIN_HEIGHT - 1)).toBe(false);
		expect(showsAgenda(AGENDA_MIN_HEIGHT)).toBe(true);
		expect(showsAgenda(Number.NaN)).toBe(false);
	});

	test('el rango de hoy dura un día', () => {
		const { from, to } = todayRange(new Date(2026, 9, 21, 15, 30));
		expect(new Date(to).getTime() - new Date(from).getTime()).toBe(24 * 3600 * 1000);
		expect(new Date(from).getHours()).toBe(0);
	});

	test('los de día completo van primero y el resto por hora', () => {
		const reply = {
			state: 'ready' as const,
			truncated: false,
			entries: [
				evento('tarde', '2026-10-21T18:00:00Z'),
				evento('todo', '2026-10-21T00:00:00Z', true),
				evento('temprano', '2026-10-21T08:00:00Z'),
			],
		};
		expect(agendaFromReply(reply).map((e) => e.event_id)).toEqual(['todo', 'temprano', 'tarde']);
	});

	test('sin respuesta lista no hay lista', () => {
		expect(agendaFromReply(null)).toEqual([]);
		expect(agendaFromReply({ state: 'denied' })).toEqual([]);
	});
});
