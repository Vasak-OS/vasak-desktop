<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton, EmptyState, ListCard, SearchField } from '@vasakgroup/vue-libvasak';
import { computed, onBeforeUnmount, onMounted, type Ref, ref, watch } from 'vue';
import FilterArea from '@/components/areas/menu/FilterArea.vue';
import MenuArea from '@/components/areas/menu/MenuArea.vue';
import CategoryMenuPill from '@/components/buttons/CategoryMenuPill.vue';
import UserMenuCard from '@/components/cards/UserMenuCard.vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
import WidgetSlot from '@/components/widgets/WidgetSlot.vue';
import { getMenuItems, openApp } from '@/services/app.service';
import { dismissMenu, openSettings, toggleSessionPopup } from '@/services/window.service';
import {
	type FocusableSearchField,
	focusMenuSearch,
	prepareMenuSearch,
	type SearchFocusAttempt,
} from '@/tools/menu-search-focus';
import { logError } from '@/utils/logger';

/**
 * El menú de aplicaciones.
 *
 * Es un applet (`windows_apps/menu.rs`): cuelga del botón del panel y lo
 * envuelve `AppletPopover`, que pone el borde, la entrada que crece desde el
 * botón y la salida. Quien lo esconde es el backend —Escape y la pérdida de
 * foco los atrapa la superficie de capa—, y la página sólo pide cerrarlo cuando
 * termina lo suyo: lanzar una aplicación.
 *
 * Antes era una ventana común y se cuidaba sola: miraba el foco de la ventana
 * de Tauri para enfocar la búsqueda y para cerrarse. Dentro de una superficie de
 * capa esa ventana es la vacía, que no gana ni pierde el foco nunca, así que las
 * dos cosas pasan ahora por los avisos del applet (`shown` y `leave`).
 */

const { t } = useI18n();

const menuData: Ref<Record<string, any>> = ref({});
const categorySelected: Ref<any> = ref('all');
const filter: Ref<string> = ref('');
const selectedIndex = ref(0);
const menuLoadFailed = ref(false);
const searchField = ref<FocusableSearchField | null>(null);
/**
 * Si el menú está a la vista.
 *
 * Arranca en `true` porque la vista se monta cuando la superficie se está
 * abriendo por primera vez, y esa vez no llega `applet-shown`: el lado y el
 * origen vienen en la ruta.
 */
const menuIsOpen = ref(true);
/** El intento de foco en curso, para poder cortarlo antes de empezar otro. */
let searchFocus: SearchFocusAttempt | null = null;

const setMenu = async () => {
	try {
		const data = await getMenuItems();
		if (!data || Object.keys(data).length === 0) {
			menuLoadFailed.value = true;
			menuData.value = {};
			return;
		}
		// Ya vienen ordenadas: el backend las ordena al armar la caché del menú,
		// que sólo se rearma cuando cambia un .desktop. Ordenarlas acá era
		// repetir novecientas comparaciones de colación en cada apertura.
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

const openConfiguration = async () => {
	try {
		await openSettings();
	} catch (error) {
		logError('Error al abrir configuración:', error);
	}
};

/**
 * El menú volvió a la vista.
 *
 * La superficie se esconde en vez de destruirse, así que la vista no se vuelve
 * a montar: esto es lo único que corre en cada apertura. Vacía la búsqueda
 * anterior y enfoca el campo — el `autofocus` del campo sólo cubre el primer
 * montaje (ver `vasak-desktop#122`).
 */
const onShown = () => {
	menuIsOpen.value = true;
	searchFocus?.cancel();
	searchFocus = prepareMenuSearch({
		field: () => searchField.value,
		clear: () => {
			filter.value = '';
		},
	});
};

/**
 * El menú se empieza a ir.
 *
 * Quedan hasta 150 ms de reintentos de foco: uno que llegue con el menú ya
 * escondido le robaría el foco a donde el usuario haya ido.
 */
const onLeave = () => {
	menuIsOpen.value = false;
	searchFocus?.cancel();
	searchFocus = null;
};

const appsOfCategory = computed(
	() => (menuData.value as any)?.[categorySelected.value]?.apps ?? []
);

const appsFiltred = computed(() => {
	const allApps = (menuData.value as any)?.all?.apps ?? [];
	const query = filter.value.toLowerCase();
	if (!query) return [];
	// Data is pre-sorted on fetch, no re-sorting needed per keystroke
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

const isMenuEmpty = computed(() => {
	return menuLoadFailed.value || Object.keys(menuData.value).length === 0;
});

let unlistenMenuChanged: UnlistenFn | undefined;

onMounted(() => {
	setMenu();
	// The surface is hidden rather than destroyed, so it is not rebuilt — and
	// re-fetched — on every open. The backend watches the application
	// directories and tells us when an app is installed or removed.
	listen('menu-items-changed', () => setMenu()).then((fn) => {
		unlistenMenuChanged = fn;
	});
	document.addEventListener('keydown', onKeydown);
});

onBeforeUnmount(() => {
	document.removeEventListener('keydown', onKeydown);
	unlistenMenuChanged?.();
	searchFocus?.cancel();
	searchFocus = null;
});

/**
 * El campo se habilita tarde, y un campo desactivado no toma el foco.
 *
 * `isMenuEmpty` arranca en verdadero —`menuData` está vacío hasta que conteste
 * `getMenuItems`, que es un comando asíncrono justamente porque con la caché
 * fría lee todos los `.desktop`—, así que el campo nace desactivado. Si esa
 * lectura tarda más que los reintentos, se agotan contra un campo que no puede
 * tomar el foco y el menú queda abierto y mudo.
 *
 * Acá **no** se vacía el filtro: para cuando el menú termina de cargar, lo que
 * haya escrito lo escribió el usuario.
 *
 * `flush: 'post'` porque lo que hace falta es que el `disabled` ya no esté en el
 * DOM, no que haya cambiado el estado.
 */
watch(
	isMenuEmpty,
	(empty) => {
		if (empty || !menuIsOpen.value) return;
		searchFocus?.cancel();
		searchFocus = focusMenuSearch({ field: () => searchField.value });
	},
	{ flush: 'post' }
);

watch(filter, () => {
	selectedIndex.value = 0;
});

watch(appsFiltred, (list) => {
	if (selectedIndex.value >= list.length) {
		selectedIndex.value = Math.max(0, list.length - 1);
	}
});

/**
 * Las flechas y Enter sobre los resultados de la búsqueda.
 *
 * Escape no va acá: lo atrapa la superficie, y si se lo queda el campo lo toma
 * `AppletPopover`. Atenderlo también acá cerraría dos veces.
 */
const onKeydown = (event: KeyboardEvent) => {
	if (!filter.value) return;

	const list = appsFiltred.value;
	if (list.length === 0) return;

	if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
		event.preventDefault();
		selectedIndex.value = (selectedIndex.value + 1) % list.length;
	} else if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
		event.preventDefault();
		selectedIndex.value = (selectedIndex.value - 1 + list.length) % list.length;
	} else if (event.key === 'Enter') {
		event.preventDefault();
		const app = list[selectedIndex.value];
		if (app?.path) {
			openApp({ path: app.path });
			void dismissMenu();
		}
	}
};
</script>

<template>
  <AppletPopover applet="menu" @shown="onShown" @leave="onLeave">
    <!-- La misma distribución de siempre, que ahora llena el applet en lugar
         de la ventana: las alturas salen de la columna y no de `100vh`, porque
         la superficie es más grande que el applet (el margen de sombra).

         Es un contenedor (`@container`) para poder achicarse sin salirse
         cuando el backend le da menos lugar que 900×620 en una pantalla chica:
         con 768 px o más —el menú de siempre— las tres zonas van lado a lado
         como siempre; por debajo se apilan y la columna se desplaza. Por el
         ancho del menú y no de la pantalla: en WebKitGTK `matchMedia` no avisa,
         y el menú no sabe en qué monitor está. -->
    <div class="@container flex h-full min-h-0 flex-col">
    <div class="mb-4 flex flex-wrap items-center justify-between gap-4">
      <UserMenuCard />

      <!-- `autofocus` cubre el primer montaje, que es el único que hay: la
           superficie se esconde en vez de destruirse. Las aperturas siguientes
           las cubre `prepareMenuSearch` desde el aviso `shown` del applet, y
           por eso el `ref` — el campo expone `focus()`, que dice si el foco
           llegó. -->
      <SearchField
        ref="searchField"
        v-model="filter"
        :label="t('components.SearchMenuComponent.placeholder')"
        :disabled="isMenuEmpty"
        autofocus
        class="min-w-48 grow" />

      <!-- Los botones de sesión son los de la librería, `ghost` y del tamaño
           de siempre (40): sin escala ni giro al pasar, el velo neutro de Once
           UI y el icono simbólico del tema, que sigue el color del texto. -->
      <div class="flex items-center gap-2">
        <ActionButton
          v-for="action in sessionActions"
          :key="action.icon"
          label=""
          :icon="action.icon"
          :icon-alt="action.title"
          :title="action.title"
          variant="ghost"
          size="lg"
          @click="action.handler"
        />
      </div>
    </div>

    <transition enter-active-class="transition-opacity duration-200 ease-ui" leave-active-class="transition-opacity duration-150 ease-ui" enter-from-class="opacity-0" leave-to-class="opacity-0" mode="out-in">
      <div v-if="isMenuEmpty" key="empty-state" class="flex flex-1 min-h-0 items-center justify-center">
        <EmptyState :title="t('views.menu.noApps')" icon="application-x-executable" />
      </div>
      <div v-else-if="filter !== ''" key="filter-view" class="flex-1 min-h-0 overflow-y-auto">
        <FilterArea :apps="appsFiltred" :selected-index="selectedIndex" />
      </div>
      <div
        v-else
        key="main-view"
        class="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto @3xl:grid-cols-3 @3xl:overflow-visible"
      >
        <!-- Las tres zonas son tarjetas de la librería (`ListCard`): la
             superficie de lo que se apoya, el canto fino y el radio `l`. -->
        <ListCard custom-class="h-72 min-h-0 @3xl:h-full">
          <div class="h-full min-w-0 flex-1 overflow-y-auto">
            <MenuArea :apps="appsOfCategory" />
          </div>
        </ListCard>

        <div class="grid min-h-0 grid-rows-[auto_18rem] gap-4 @3xl:col-span-2 @3xl:h-full @3xl:grid-rows-[1fr_2fr]">
          <ListCard custom-class="min-h-0">
            <div class="grid h-full min-h-0 min-w-0 flex-1 grid-cols-[1fr_2fr] gap-3">
              <div v-if="categoryEntries.all" role="menu" :aria-label="t('views.menu.categories')" class="flex min-h-14 items-center justify-center">
                <CategoryMenuPill
                  :category="categoryEntries.all[0]"
                  :image="categoryEntries.all[1].icon"
                  :label="t(categoryEntries.all[1].description)"
                  v-model:categorySelected="categorySelected"
                  large
                />
              </div>

              <div role="menu" :aria-label="t('views.menu.categories')" class="grid min-h-0 grid-cols-3 grid-rows-2 gap-3">
                <CategoryMenuPill
                  v-for="([key, value]) in categoryEntries.others.slice(0, 6)"
                  :key="key"
                  :category="key"
                  :image="value.icon"
                  :label="t(value.description)"
                  v-model:categorySelected="categorySelected"
                />
              </div>
            </div>
          </ListCard>

          <!-- El hueco de la derecha acepta cualquiera de los widgets del
               escritorio: cambiar `type` alcanza. El marco y el contenedor los
               pone WidgetSlot, que es lo que hace que las medidas de adentro se
               resuelvan contra este hueco y no contra la ventana entera. -->
          <div class="min-h-0">
            <WidgetSlot type="weather" />
          </div>
        </div>
      </div>
    </transition>
    </div>
  </AppletPopover>
</template>
