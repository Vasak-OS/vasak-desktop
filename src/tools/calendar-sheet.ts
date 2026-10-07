/**
 * Las cuentas del calendario del centro de control (vasak-desktop#183), sin Vue.
 *
 * El calendario se abre tocando la fecha de la tarjeta de usuario y muestra el
 * mes con los días que tienen eventos. Los eventos salen del mismo lugar que
 * los del tablero de fecha —el almacén local de vasak-accounts, por
 * `calendar.service.ts`— y pasan por las mismas cuentas (`toCalendarEntry`,
 * `markedDates`): un evento flotante o de día completo cae en el mismo día en
 * los dos lados.
 */
import { type IsoDate, markedDates, weekStartOf } from '@vasakgroup/vue-libvasak';
import type { CalendarReply } from '@/services/calendar.service';
import { toCalendarEntry } from '@/tools/date-board';

/**
 * El idioma de la interfaz como etiqueta BCP 47: `es_AR.UTF-8` → `es-AR`.
 *
 * El plugin de idiomas contesta `es` o `en-US`, pero un idioma que venga del
 * entorno trae guion bajo y codificación, e `Intl` lo rechaza. Sin idioma,
 * `undefined`: el del sistema.
 */
export function localeTag(locale: string | null | undefined): string | undefined {
	const tag = (locale ?? '').split('.')[0]?.split('@')[0]?.replaceAll('_', '-').trim();
	if (!tag) return undefined;
	try {
		return Intl.getCanonicalLocales(tag)[0];
	} catch {
		return undefined;
	}
}

/**
 * El primer día de la semana para el idioma de la interfaz: 0 domingo … 6
 * sábado. Es el de `MonthCalendar` (`weekStartOf`), que lo lee de CLDR: `es`
 * empieza en lunes y `en`, que es el de Estados Unidos, en domingo.
 */
export function weekStartFor(locale: string | null | undefined): number {
	return weekStartOf(localeTag(locale));
}

/**
 * Los días a marcar según lo que contestó el almacén.
 *
 * Sólo una respuesta con eventos marca algo. Sin el servicio, sin permiso o
 * con un error el calendario muestra sólo el mes: no es un tablero de
 * eventos, y avisar «no disponible» en el centro sería ruido.
 */
export function marksFromReply(reply: CalendarReply | null | undefined): IsoDate[] {
	if (reply?.state !== 'ready') return [];
	return markedDates(reply.entries.map((occurrence) => toCalendarEntry(occurrence)));
}
