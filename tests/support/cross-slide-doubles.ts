/**
 * Los dobles de la vista del centro de control para las pruebas del
 * cross-slide: los del calendario, más dos que dejan ver lo que la estructura
 * nueva garantiza.
 *
 * - `NotificationArea` deja pasar las clases y los `data-*` que le pone la
 *   vista (la lista debe tener su alto mínimo y su marca) y cuenta cuántas
 *   veces se monta: si el cambio A↔B la desmontara, la cuenta de
 *   notificaciones se perdería.
 * - `MusicWidget` avisa `presence` desde un botón, como el de verdad cuando
 *   aparece un reproductor.
 */
import { defineComponent, h, onMounted } from 'vue';

export * from './calendar-doubles';

export const lifecycle = { notificationAreaMounts: 0 };

export const NotificationArea = defineComponent({
	name: 'NotificationAreaDouble',
	emits: ['summary'],
	setup() {
		onMounted(() => {
			lifecycle.notificationAreaMounts++;
		});
		return () => h('div', { 'data-fake-notifications': '' });
	},
});

export const MusicWidget = defineComponent({
	name: 'MusicWidgetDouble',
	emits: ['presence'],
	setup(_props, { emit }) {
		return () =>
			h('button', { type: 'button', 'data-fake-music': '', onClick: () => emit('presence', true) }, 'música');
	},
});
