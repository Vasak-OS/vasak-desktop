/**
 * Las cuentas del widget de calendario (vasak-desktop#269), sin Vue.
 *
 * Lee del mismo almacén que el tablero de fecha y el calendario del centro de
 * control (`calendar.service.ts`), y pasa por las mismas cuentas: lo que
 * marca un día allá lo marca acá.
 */
import { type IsoDate, toIsoDate } from '@vasakgroup/vue-libvasak';
import type { CalendarOccurrence, CalendarReply } from '@/services/calendar.service';

/** Cuánto alto pide la lista de hoy, debajo del mes: cuatro filas de la cuadrícula. */
export const AGENDA_MIN_HEIGHT = 500;

/** Si la caja tiene lugar para la lista de hoy debajo del mes. */
export function showsAgenda(height: number): boolean {
	return Number.isFinite(height) && height >= AGENDA_MIN_HEIGHT;
}

/** El rango `[hoy, mañana)` en el formato que pide el almacén. */
export function todayRange(today: Date): { from: string; to: string } {
	const start = new Date(today.getFullYear(), today.getMonth(), today.getDate());
	const end = new Date(start);
	end.setDate(end.getDate() + 1);
	return { from: start.toISOString(), to: end.toISOString() };
}

/** Los eventos de hoy en orden; los de día completo primero. Vacío sin respuesta lista. */
export function agendaFromReply(reply: CalendarReply | null | undefined): CalendarOccurrence[] {
	if (reply?.state !== 'ready') return [];
	return [...reply.entries].sort(
		(a, b) => Number(b.all_day) - Number(a.all_day) || a.start.localeCompare(b.start)
	);
}

/** Hoy como fecha ISO local, para el calendario. */
export function todayIso(now = new Date()): IsoDate {
	return toIsoDate(now);
}
