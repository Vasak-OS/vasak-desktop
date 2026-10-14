import { invoke } from '@tauri-apps/api/core';
import { CHANGED_EVENT } from '@vasakgroup/plugin-vsk-store';

/**
 * Los eventos del calendario, del almacén local de vasak-accounts.
 *
 * Es la única fuente de eventos del escritorio (decisión 6 del taller): el
 * servicio que mantiene la copia local de los calendarios contesta desde lo
 * guardado. Lo hace `commands/calendar.rs`; acá sólo se cruza el IPC.
 *
 * **Todo lo que muestre eventos pasa por acá**: el tablero del panel, el
 * calendario del centro de control y los widgets del escritorio. Así leen lo
 * mismo, de la misma forma, y se enteran juntos de lo nuevo
 * ([`CALENDAR_CHANGED`]).
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

/**
 * Los cuatro desenlaces de volver a pedir el permiso del calendario.
 *
 * - `reopened`: se olvidó la decisión anterior; el próximo pedido de eventos
 *   reabre el diálogo, así que hay que releer.
 * - `too_soon`: se volvió a pedir hace muy poco (el techo contra la fatiga de
 *   permisos); hay que esperar un momento.
 * - `not_allowed`: el escritorio no puede pedir este recurso (no debería pasar).
 * - `failed`: el servicio de permisos no está, o falló.
 */
export type ReaskOutcome =
	| { outcome: 'reopened' }
	| { outcome: 'too_soon' }
	| { outcome: 'not_allowed' }
	| { outcome: 'failed'; detail: string };

/**
 * Le pide a `vasak-permissions` que vuelva a preguntar por el permiso de leer el
 * calendario guardado (ola 2 de «pedir permiso de nuevo»).
 *
 * Un rechazo recordado no se reabre con sólo reintentar la lectura: hace falta
 * que el servicio olvide la decisión del escritorio sobre `store.calendar`. Tras
 * `reopened`, releer con [`calendarOccurrences`] dispara el diálogo nuevo. Lo
 * hace `commands/store_access.rs`.
 */
export const requestCalendarAccessAgain = (): Promise<ReaskOutcome> =>
	invoke<ReaskOutcome>('request_calendar_access_again');

/**
 * El evento de que cambió lo guardado del calendario: el sincronizador trajo
 * algo nuevo en segundo plano. Quien muestra eventos lo escucha con
 * `useSharedEvent(CALENDAR_CHANGED, …)` —un solo oyente de Tauri para todo el
 * escritorio— y relee su rango; no hace falta preguntar a intervalos. Las
 * señales de una misma vuelta llegan juntas, en un solo aviso. Lo emite
 * `tauri-plugin-vsk-store`, que el escritorio registra sólo para el área del
 * calendario y que antes de creerle nada comprueba que el dueño del nombre
 * del sincronizador sea el de verdad.
 */
export const CALENDAR_CHANGED = CHANGED_EVENT;
