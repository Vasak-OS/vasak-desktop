import { invoke } from '@tauri-apps/api/core';

/**
 * Los eventos del calendario, del almacén local de vasak-accounts.
 *
 * Es la única fuente de eventos del escritorio (decisión 6 del taller): el
 * servicio que mantiene la copia local de los calendarios contesta desde lo
 * guardado. Lo hace `commands/calendar.rs`; acá sólo se cruza el IPC.
 */

/** Una vez de un evento, tal como la manda `ListOccurrences` (RFC 3339 en UTC). */
export interface CalendarOccurrence {
	event_id: string;
	occurrence_id: string;
	calendar_id: string;
	title: string;
	start: string;
	end: string;
	all_day: boolean;
	floating: boolean;
	color: string | null;
	/** El nombre del calendario, de `ListCalendars`. */
	calendar: string | null;
}

/** Las cuatro respuestas que el tablero sabe dibujar. */
export type CalendarReply =
	| { state: 'ready'; entries: CalendarOccurrence[]; truncated: boolean }
	| { state: 'unavailable' }
	| { state: 'denied' }
	| { state: 'failed'; detail: string };

/**
 * Las veces de los eventos en `[from, to)`.
 *
 * La primera vez puede tardar lo que tarde la persona en contestar el diálogo
 * de permiso: la promesa se queda esperando, no falla.
 */
export const calendarOccurrences = (from: string, to: string): Promise<CalendarReply> =>
	invoke<CalendarReply>('calendar_occurrences', { from, to });

/** El lugar de cada evento, en el mismo orden; vacío si no tiene. */
export const calendarLocations = (
	events: Array<Pick<CalendarOccurrence, 'event_id' | 'occurrence_id'>>
): Promise<string[]> =>
	invoke<string[]>('calendar_locations', {
		events: events.map(({ event_id, occurrence_id }) => ({ event_id, occurrence_id })),
	});

/** Abre `vasak-calendar`. */
export const openCalendar = (): Promise<void> => invoke<void>('open_calendar');
