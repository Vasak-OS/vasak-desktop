/**
 * Los dobles del calendario del centro de control (vasak-desktop#183).
 *
 * `CalendarSheet.vue` y `UserControlCenterCard.vue` se compilan con sus
 * `@/…` mandados acá (`loadComponent`). Lo puro es el módulo de verdad; el
 * servicio de eventos es un doble que la prueba controla desde `calendar`.
 *
 * Para la vista entera (`ControlCenterView.vue`) cada componente del
 * escritorio que importa es un doble con su nombre: la prueba cambia
 * `import X from '@/….vue'` por `import { X } from` este módulo.
 */
import { defineComponent, h } from 'vue';
import type { CalendarReply } from '../../src/services/calendar.service';

export { localeTag, marksFromReply, weekStartFor } from '../../src/tools/calendar-sheet';
export { modeOnCountChange, modeOnOpen, summaryText } from '../../src/tools/control-center-mode';
export { gridRange } from '../../src/tools/date-board';
export { capitalizeFirst } from '../../src/tools/text-case';

type Answer = CalendarReply | Error | (() => Promise<CalendarReply>);

/** Lo que contesta el servicio y lo que le pidieron. */
export const calendar = {
	answer: { state: 'unavailable' } as Answer,
	calls: [] as Array<{ from: string; to: string }>,
	errors: [] as unknown[],
	reset() {
		this.answer = { state: 'unavailable' };
		this.calls = [];
		this.errors = [];
	},
};

export async function calendarOccurrences(from: string, to: string): Promise<CalendarReply> {
	calendar.calls.push({ from, to });
	const answer = calendar.answer;
	if (answer instanceof Error) throw answer;
	if (typeof answer === 'function') return answer();
	return answer;
}

export const logError = (_message: string, data?: unknown) => {
	calendar.errors.push(data);
};

// ── La vista ────────────────────────────────────────────────────────────────

export const sharedEvents = new Map<string, () => void>();
export function useSharedEvent(name: string, handler: () => void) {
	sharedEvents.set(name, handler);
}
export const hideControlCenter = async () => {};
export const toggleSessionPopup = async () => {};

const stub = (name: string) =>
	defineComponent({ name, inheritAttrs: false, setup: () => () => h('div', { [`data-stub-${name}`]: '' }) });

/** El doble mudo de un componente que esta prueba no conoce. */
export const silent = stub;

/** La tarjeta: la fecha es un botón que avisa, como la de verdad. */
export const UserControlCenterCard = defineComponent({
	name: 'UserControlCenterCardDouble',
	props: { calendarOpen: { type: Boolean, default: false } },
	emits: ['open-calendar'],
	setup(props, { emit }) {
		return () =>
			h('div', { 'data-user-card': '' }, [
				h(
					'button',
					{
						type: 'button',
						'data-user-date': '',
						'aria-expanded': String(props.calendarOpen),
						onClick: () => emit('open-calendar'),
					},
					'martes, 6 de octubre'
				),
			]);
	},
});

/** La ficha: «Volver» avisa `back`. */
export const CalendarSheet = defineComponent({
	name: 'CalendarSheetDouble',
	emits: ['back'],
	setup(_props, { emit }) {
		return () =>
			h('section', { 'data-calendar-sheet': '' }, [
				h('button', { type: 'button', 'data-calendar-back': '', onClick: () => emit('back') }, 'Volver'),
			]);
	},
});

export const NotificationArea = defineComponent({
	name: 'NotificationAreaDouble',
	emits: ['summary'],
	setup: () => () => h('div', { 'data-fake-notifications': '' }),
});
export const QuickSettingsPanel = stub('QuickSettingsPanel');
export const PhoneControlCenterCard = stub('PhoneControlCenterCard');
export const BluetoothControl = stub('BluetoothControl');
export const BrightnessControl = stub('BrightnessControl');
export const DoNotDisturbToggle = stub('DoNotDisturbToggle');
export const NetworkControl = stub('NetworkControl');
export const ThemeToggle = stub('ThemeToggle');
export const VolumeControl = stub('VolumeControl');
export const MusicWidget = stub('MusicWidget');
