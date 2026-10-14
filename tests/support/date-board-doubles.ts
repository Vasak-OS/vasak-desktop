/**
 * Los dobles del tablero de fecha (`DateBoardAppletView.vue`, vasak-desktop#130).
 *
 * La vista se compila con sus `@/…` mandados acá (`loadComponent`): el
 * `default` es la ventana del applet —sólo dibuja lo de adentro—, el servicio
 * de eventos es un doble que la prueba controla desde `board`, y las cuentas
 * del tablero son las de verdad.
 */
import { defineComponent, h, ref } from 'vue';
import type { CalendarOccurrence, CalendarReply, ReaskOutcome } from '../../src/services/calendar.service';

export {
	dayWeather,
	gridRange,
	occurrenceKey,
	relativeDayLabel,
	toCalendarEntry,
	upcomingHours,
	weatherIcon,
	weatherRings,
} from '../../src/tools/date-board';

/** Lo que contesta el servicio y lo que le pidieron. */
export const board = {
	answer: { state: 'unavailable' } as CalendarReply,
	location: 'Sala 3',
	calls: [] as Array<{ from: string; to: string }>,
	locationCalls: [] as string[][],
	/** Si está, el próximo pedido de lugares espera a que se cumpla. */
	locationGate: null as Promise<void> | null,
	/** Lo que contesta volver a pedir el permiso, y cuántas veces lo pidieron. */
	reaskOutcome: { outcome: 'reopened' } as ReaskOutcome,
	reaskCalls: 0,
	reset() {
		this.locationGate = null;
		this.answer = { state: 'unavailable' };
		this.location = 'Sala 3';
		this.calls = [];
		this.locationCalls = [];
		this.reaskOutcome = { outcome: 'reopened' };
		this.reaskCalls = 0;
	},
};

export const CALENDAR_CHANGED = 'calendar-changed';

export async function calendarOccurrences(from: string, to: string): Promise<CalendarReply> {
	board.calls.push({ from, to });
	return board.answer;
}

export async function calendarLocations(events: Array<Pick<CalendarOccurrence, 'event_id'>>): Promise<string[]> {
	board.locationCalls.push(events.map((event) => event.event_id));
	// El lugar es el de cuando se pidió, aunque la respuesta tarde.
	const value = board.location;
	const gate = board.locationGate;
	board.locationGate = null;
	if (gate) await gate;
	return events.map(() => value);
}

export const openCalendar = async () => {};

export async function requestCalendarAccessAgain(): Promise<ReaskOutcome> {
	board.reaskCalls += 1;
	return board.reaskOutcome;
}

export const sharedEvents = new Map<string, () => void>();
export function useSharedEvent(name: string, handler: () => void) {
	sharedEvents.set(name, handler);
}

/** Sin clima: el tablero lo dibuja vacío, que acá no se mira. */
export function useWeather() {
	return { weather: ref(null), failed: ref(false), loading: ref(false) };
}

export const logError = () => {};

/** La ventana del applet: sólo lo de adentro. */
export default defineComponent({
	name: 'AppletPopoverDouble',
	inheritAttrs: false,
	emits: ['shown', 'leave'],
	setup:
		(_props, { slots }) =>
		() =>
			h('div', { 'data-applet-popover': '' }, slots.default?.({ close: () => {} })),
});
