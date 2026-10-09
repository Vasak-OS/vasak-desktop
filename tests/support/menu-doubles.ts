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
import { computed, defineComponent, getCurrentInstance, h, ref } from 'vue';

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

/**
 * La configuración del menú: la variante compacta de fábrica, con todas las
 * opciones en su valor por omisión. Alcanza para que `MenuView` dibuje su marco
 * de siempre y resuelva la variante a su layout (que acá es una caja).
 */
export function useMenuConfig() {
	return {
		menu: ref({
			variant: 'compact',
			widget: 'weather',
			showUser: true,
			showSessionActions: true,
			searchPosition: 'top',
			showPlaces: false,
			favorites: [] as string[],
			showFavorites: false,
			header: 'none',
			headerImage: '',
			headerStrength: 60,
			showGreeting: true,
			showWeather: true,
		}),
		setMenuConfig: async () => {},
		toggleFavoritePath: async () => {},
	};
}

/**
 * El estado compartido del menú, doblado: usa `menu.items` de arriba. Lo que la
 * prueba del foco necesita es que `isMenuEmpty` arranque en verdadero —el campo
 * nace desactivado— y pase a falso cuando se carga el menú.
 */
export function useMenuController() {
	const menuData = ref<Record<string, any>>({});
	const categorySelected = ref('all');
	const filter = ref('');
	const selectedIndex = ref(0);
	const menuLoadFailed = ref(false);

	const setMenu = async () => {
		const data = await getMenuItems();
		if (!data || Object.keys(data).length === 0) {
			menuLoadFailed.value = true;
			menuData.value = {};
			return;
		}
		menuData.value = data;
		menuLoadFailed.value = false;
	};

	const isMenuEmpty = computed(
		() => menuLoadFailed.value || Object.keys(menuData.value).length === 0
	);

	return {
		menuData,
		categorySelected,
		filter,
		selectedIndex,
		menuLoadFailed,
		isMenuEmpty,
		appsOfCategory: computed(() => menuData.value?.[categorySelected.value]?.apps ?? []),
		appsFiltred: computed(() => {
			const all = menuData.value?.all?.apps ?? [];
			const query = filter.value.toLowerCase();
			if (!query) return [];
			return all.filter(
				(app: any) =>
					app.name.toLowerCase().includes(query) ||
					app.description.toLowerCase().includes(query)
			);
		}),
		categoryEntries: computed(() => ({ all: ['all', menuData.value.all], others: [] })),
		sessionActions: computed(() => [] as Array<{ title: string; icon: string; handler: () => void }>),
		setMenu,
	};
}

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
