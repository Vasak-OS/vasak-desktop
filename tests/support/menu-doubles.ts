/**
 * Los dobles de lo que importa `MenuView.vue` desde `@/…`, para probar que la
 * búsqueda queda enfocada cada vez que el menú se abre (vasak-desktop#151).
 *
 * Lo que decide el foco —`menu-search-focus.ts`— es el módulo de verdad, y el
 * campo es el `SearchField` publicado de la librería. Las piezas de alrededor
 * (la tarjeta del usuario, las categorías, el widget) son una caja vacía, y
 * `AppletPopover` es la caja con la ranura que avisa `shown` cuando la prueba
 * dice que el applet volvió a la vista: lo mismo que hace la de verdad con el
 * aviso `applet-shown` del backend, venga la apertura del botón del panel o de
 * la tecla Super.
 */
import { defineComponent, getCurrentInstance, h } from 'vue';

export { resolveMenuKey } from '../../src/tools/menu-keyboard';
export { focusMenuSearch, prepareMenuSearch } from '../../src/tools/menu-search-focus';

type Emitter = (event: 'shown' | 'leave') => void;

export const menu = {
	emitters: [] as Emitter[],
	items: {
		all: { name: 'Todas', apps: [{ name: 'Firefox', description: 'Navegador', path: '/f', icon: 'firefox' }] },
	} as Record<string, unknown>,
	/** El backend volvió a mostrar el applet. */
	show() {
		for (const emit of this.emitters) emit('shown');
	},
	/** El applet empezó a irse. */
	leave() {
		for (const emit of this.emitters) emit('leave');
	},
	reset() {
		this.emitters.length = 0;
	},
};

export const getMenuItems = async () => menu.items;
export const openApp = async () => {};
export const dismissMenu = async () => {};
export const openSettings = async () => {};
export const toggleSessionPopup = () => {};
export const logError = () => {};

/** Todas las piezas `default` de la vista: una caja con su ranura. */
export default defineComponent({
	name: 'MenuPieceDouble',
	inheritAttrs: false,
	emits: ['shown', 'leave'],
	setup(_props, { slots, emit }) {
		if (getCurrentInstance()) menu.emitters.push(emit as Emitter);
		return () => h('div', slots.default?.({ close: () => {} }));
	},
});
