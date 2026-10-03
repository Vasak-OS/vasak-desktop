/**
 * El puente con los comandos del calendario (`commands/calendar.rs`): qué
 * comando se llama y con qué argumentos, que es lo que se separa sin avisar
 * cuando se renombra un lado y no el otro.
 */
import { afterAll, beforeAll, describe, expect, test } from 'bun:test';

const calls: Array<{ cmd: string; args: Record<string, unknown> }> = [];
let service: typeof import('@/services/calendar.service');

beforeAll(async () => {
	(globalThis as any).window = {
		__TAURI_INTERNALS__: {
			transformCallback: () => 1,
			invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
				calls.push({ cmd, args });
				if (cmd === 'calendar_occurrences') return { state: 'denied' };
				if (cmd === 'calendar_locations') return ['Sala 3'];
				return null;
			},
		},
	};
	service = await import('@/services/calendar.service');
});

afterAll(() => {
	delete (globalThis as any).window;
});

describe('el servicio del calendario', () => {
	test('las veces de un rango van a calendar_occurrences y vuelven tal cual', async () => {
		const reply = await service.calendarOccurrences(
			'2026-02-23T03:00:00.000Z',
			'2026-04-06T03:00:00.000Z'
		);

		expect(reply).toEqual({ state: 'denied' });
		expect(calls.at(-1)).toEqual({
			cmd: 'calendar_occurrences',
			args: { from: '2026-02-23T03:00:00.000Z', to: '2026-04-06T03:00:00.000Z' },
		});
	});

	test('el lugar se pide sólo con los dos identificadores de cada vez', async () => {
		const found = await service.calendarLocations([
			{ event_id: 'cuenta/7', occurrence_id: '3', title: 'no viaja' } as never,
		]);

		expect(found).toEqual(['Sala 3']);
		expect(calls.at(-1)).toEqual({
			cmd: 'calendar_locations',
			args: { events: [{ event_id: 'cuenta/7', occurrence_id: '3' }] },
		});
	});

	test('abrir el calendario es open_calendar, sin argumentos', async () => {
		await service.openCalendar();

		expect(calls.at(-1)?.cmd).toBe('open_calendar');
	});
});
