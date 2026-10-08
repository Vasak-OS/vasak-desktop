import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, type Ref, ref, watch } from 'vue';
import { getMenuItems } from '@/services/app.service';
import { openSettings, toggleSessionPopup } from '@/services/window.service';
import { logError } from '@/utils/logger';

/**
 * El estado y la lógica que comparten todas las variantes del menú de inicio
 * (vasak-desktop#203).
 *
 * Antes vivía entero dentro de `MenuView.vue`. Con cinco esqueletos que dibujan
 * las mismas aplicaciones de formas distintas, lo que no cambia —leer el menú,
 * filtrar, las categorías, los botones de sesión— se saca acá para que cada
 * layout sólo se ocupe de **cómo** se ve. No tiene ciclo de vida ni teclado: eso
 * lo maneja `MenuView`, que es la que vive dentro del applet; acá sólo hay
 * estado reactivo y lo derivado de él, para poder usarlo desde cualquier
 * variante.
 */
export function useMenuController() {
	const { t } = useI18n();

	const menuData: Ref<Record<string, any>> = ref({});
	const categorySelected: Ref<string> = ref('all');
	const filter = ref('');
	const selectedIndex = ref(0);
	const menuLoadFailed = ref(false);

	const setMenu = async () => {
		try {
			const data = await getMenuItems();
			if (!data || Object.keys(data).length === 0) {
				menuLoadFailed.value = true;
				menuData.value = {};
				return;
			}
			// Ya vienen ordenadas: el backend las ordena al armar la caché.
			menuData.value = data;
			menuLoadFailed.value = false;
		} catch (error) {
			logError('Error al cargar el menú:', error);
			menuLoadFailed.value = true;
			menuData.value = {};
		}
	};

	const openSessionPopup = (action: string) => {
		toggleSessionPopup(action);
	};

	const openConfiguration = async () => {
		try {
			await openSettings();
		} catch (error) {
			logError('Error al abrir configuración:', error);
		}
	};

	/** Los cinco botones de sesión, en el orden de siempre. */
	const sessionActions = computed(() => [
		{
			title: t('views.menu.configuration'),
			icon: 'settings',
			handler: () => void openConfiguration(),
		},
		{
			title: t('views.menu.shutdown'),
			icon: 'system-shutdown',
			handler: () => openSessionPopup('shutdown'),
		},
		{
			title: t('views.menu.reboot'),
			icon: 'system-reboot',
			handler: () => openSessionPopup('reboot'),
		},
		{
			title: t('views.menu.logout'),
			icon: 'system-log-out',
			handler: () => openSessionPopup('logout'),
		},
		{
			title: t('views.menu.suspend'),
			icon: 'system-suspend',
			handler: () => openSessionPopup('suspend'),
		},
	]);

	const appsOfCategory = computed(() => menuData.value?.[categorySelected.value]?.apps ?? []);

	const appsFiltred = computed(() => {
		const allApps = menuData.value?.all?.apps ?? [];
		const query = filter.value.toLowerCase();
		if (!query) return [];
		return allApps.filter(
			(app: any) =>
				app.name.toLowerCase().includes(query) || app.description.toLowerCase().includes(query)
		);
	});

	const categoryEntries = computed(() => {
		const entries = Object.entries(menuData.value as Record<string, any>);
		const allIdx = entries.findIndex(([k]) => k === 'all');
		const all = allIdx >= 0 ? entries.splice(allIdx, 1)[0] : entries.shift();
		return { all, others: entries };
	});

	const isMenuEmpty = computed(
		() => menuLoadFailed.value || Object.keys(menuData.value).length === 0
	);

	watch(filter, () => {
		selectedIndex.value = 0;
	});

	watch(appsFiltred, (list) => {
		if (selectedIndex.value >= list.length) {
			selectedIndex.value = Math.max(0, list.length - 1);
		}
	});

	return {
		menuData,
		categorySelected,
		filter,
		selectedIndex,
		menuLoadFailed,
		isMenuEmpty,
		appsOfCategory,
		appsFiltred,
		categoryEntries,
		sessionActions,
		setMenu,
	};
}

/** Lo que `useMenuController` devuelve, para tipar lo que reciben las variantes. */
export type MenuController = ReturnType<typeof useMenuController>;
