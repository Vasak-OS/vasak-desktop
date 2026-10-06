/**
 * Los dobles de lo que importan `QuickSettingsPanel.vue` y `NotificationArea.vue`
 * desde `@/…` (vasak-desktop#175). Lo puro es el módulo de verdad; los
 * mosaicos, las fichas y el servicio de notificaciones son dobles que la prueba
 * controla desde `center`.
 */
import { defineComponent, h, ref } from 'vue';
import type { Notification } from '../../src/interfaces/notifications';
import type { TileSpec } from '../../src/tools/control-center-tiles';

export { availableTiles } from '../../src/tools/control-center-tiles';
export { groupNotifications } from '../../src/tools/notifications';

/** Un mosaico de mentira: la flecha del detalle emite `open`, como los de verdad. */
function fakeTile(id: string) {
	return defineComponent({
		name: `FakeTile-${id}`,
		emits: ['open'],
		setup(_props, { emit }) {
			return () =>
				h('button', { type: 'button', 'data-tile-detail': '', 'data-fake-tile': id, onClick: () => emit('open') }, id);
		},
	});
}

function fakeDetail(id: string) {
	return defineComponent({
		name: `FakeDetail-${id}`,
		inheritAttrs: false,
		setup(_props, { attrs }) {
			return () => h('div', { 'data-fake-detail': id, 'data-attrs': JSON.stringify(attrs) }, id);
		},
	});
}

export const CONTROL_CENTER_TILES: readonly TileSpec[] = [
	{ id: 'network', tile: fakeTile('network'), detail: fakeDetail('network'), detailProps: { hideX: true } },
	{ id: 'bluetooth', tile: fakeTile('bluetooth'), detail: fakeDetail('bluetooth'), requires: 'bluetooth' },
	{ id: 'theme', tile: fakeTile('theme') },
	{ id: 'screen-time', tile: fakeTile('screen-time') },
	{ id: 'search', tile: fakeTile('search') },
];

/** Lo que la prueba pone y lo que mira. */
export const center = {
	notifications: ref<Notification[]>([]),
	handlers: new Map<string, (payload: unknown) => void>(),
	reset() {
		this.notifications.value = [];
		this.handlers.clear();
	},
};

export const getAllNotifications = async () => center.notifications.value;
export const deleteNotification = async () => {};
export const clearNotifications = async () => {};

export function useSharedEvent(name: string, handler: (payload: unknown) => void) {
	center.handlers.set(name, handler);
}

/** `NotificationGroupCard`, sin dibujo: alcanza con que exista. */
export default defineComponent({
	name: 'NotificationGroupCardDouble',
	props: { group: { type: Object, required: true } },
	setup(props) {
		return () => h('div', { 'data-group': (props.group as { app_name: string }).app_name });
	},
});
