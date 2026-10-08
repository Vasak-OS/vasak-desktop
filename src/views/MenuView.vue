<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */
/** biome-ignore-all lint/correctness/noUnusedVariables: usados en la plantilla */
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { EmptyState, SearchField } from '@vasakgroup/vue-libvasak';
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import FilterArea from '@/components/areas/menu/FilterArea.vue';
import CompactMenu from '@/components/areas/menu/layouts/CompactMenu.vue';
import MenuHero from '@/components/areas/menu/MenuHero.vue';
import MenuSessionActions from '@/components/areas/menu/MenuSessionActions.vue';
import UserMenuCard from '@/components/cards/UserMenuCard.vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
import { openApp } from '@/services/app.service';
import { dismissMenu } from '@/services/window.service';
import { useMenuConfig } from '@/tools/composables/useMenuConfig';
import { type MenuController, useMenuController } from '@/tools/composables/useMenuController';
import type { MenuVariant } from '@/tools/menu-config';
import { resolveMenuKey } from '@/tools/menu-keyboard';
import {
	type FocusableSearchField,
	focusMenuSearch,
	prepareMenuSearch,
	type SearchFocusAttempt,
} from '@/tools/menu-search-focus';

/**
 * El menú de aplicaciones.
 *
 * Es un applet (`windows_apps/menu.rs`): cuelga del botón del panel y lo
 * envuelve `AppletPopover`, que pone el borde, la entrada que crece desde el
 * botón y la salida. Quien lo esconde es el backend —Escape y la pérdida de
 * foco los atrapa la superficie de capa—, y la página sólo pide cerrarlo cuando
 * termina lo suyo: lanzar una aplicación.
 *
 * # Switch de esqueletos (vasak-desktop#203)
 *
 * El menú es configurable. Esta vista es el marco común a todas las variantes:
 * el encabezado (hero opcional, usuario, buscador y acciones de sesión), los
 * estados de búsqueda y de menú vacío, el teclado sobre los resultados y el
 * foco de la búsqueda. Lo que cambia entre variantes es **la vista ociosa** —la
 * disposición de las aplicaciones cuando no se está buscando—, que se resuelve a
 * un componente de `layouts/` con `<component :is>` según `menu.variant`. El
 * estado que comparten todas (`useMenuController`) se les pasa como propiedad.
 *
 * Como `App.vue` recarga la configuración en cada `config-changed`, cambiar la
 * variante o una opción en Configuración rehace el menú sin reiniciar nada.
 *
 * El foco: la superficie se **esconde** en vez de destruirse, así que el
 * `autofocus` del campo sólo cubre el primer montaje; las siguientes aperturas
 * las cubre `prepareMenuSearch` desde el aviso `shown` del applet.
 */
const { t } = useI18n();
const { menu } = useMenuConfig();
const controller: MenuController = useMenuController();
const { filter, selectedIndex, isMenuEmpty, appsFiltred, sessionActions } = controller;

/**
 * Qué componente dibuja la vista ociosa de cada variante. Las que todavía no
 * tienen esqueleto propio caen al compacto, que es el de siempre.
 */
const LAYOUTS: Partial<Record<MenuVariant, typeof CompactMenu>> = {
	compact: CompactMenu,
};
const layoutComponent = computed(() => LAYOUTS[menu.value.variant] ?? CompactMenu);

const searchField = ref<FocusableSearchField | null>(null);

/**
 * Si el menú está a la vista. Arranca en `true` porque la vista se monta cuando
 * la superficie se está abriendo por primera vez, y esa vez no llega
 * `applet-shown`.
 */
const menuIsOpen = ref(true);
/** El intento de foco en curso, para poder cortarlo antes de empezar otro. */
let searchFocus: SearchFocusAttempt | null = null;

/**
 * El menú volvió a la vista: vacía la búsqueda anterior y enfoca el campo. Es lo
 * único que corre en cada apertura (la superficie se esconde, no se destruye).
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
 * El menú se empieza a ir: corta los reintentos de foco pendientes, para que uno
 * tardío no le robe el foco a donde el usuario haya ido.
 */
const onLeave = () => {
	menuIsOpen.value = false;
	searchFocus?.cancel();
	searchFocus = null;
};

let unlistenMenuChanged: UnlistenFn | undefined;

onMounted(() => {
	controller.setMenu();
	// La superficie se esconde en vez de destruirse, así que no se rearma —ni se
	// vuelve a pedir— en cada apertura. El backend avisa cuando se instala o se
	// quita una aplicación.
	listen('menu-items-changed', () => controller.setMenu()).then((fn) => {
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
 * El campo se habilita tarde, y un campo desactivado no toma el foco: cuando el
 * menú termina de cargar con el menú ya abierto, se vuelve a enfocar. Acá no se
 * vacía el filtro: lo que haya escrito lo escribió el usuario.
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

/**
 * Las flechas y Enter sobre los resultados de la búsqueda. Escape no va acá: lo
 * atrapa la superficie. Un Enter que ya atendió el resultado enfocado tampoco:
 * ver `resolveMenuKey`.
 */
const onKeydown = (event: KeyboardEvent) => {
	if (!filter.value) return;

	const list = appsFiltred.value;
	const action = resolveMenuKey(event, list.length, selectedIndex.value);
	if (action.kind === 'none') return;

	event.preventDefault();
	if (action.kind === 'move') {
		selectedIndex.value = action.index;
		return;
	}

	const app = list[action.index];
	if (app?.path) {
		openApp({ path: app.path });
		void dismissMenu();
	}
};
</script>

<template>
  <AppletPopover applet="menu" @shown="onShown" @leave="onLeave">
    <!-- El marco común a todas las variantes. Es un contenedor (`@container`)
         para poder achicarse sin salirse cuando el backend le da menos lugar que
         900×620: por el ancho del menú y no de la pantalla (en WebKitGTK
         `matchMedia` no avisa). -->
    <div class="@container flex h-full min-h-0 flex-col">
      <MenuHero v-if="menu.header === 'hero'" :menu="menu" />

      <div
        v-if="menu.searchPosition === 'top' || menu.showUser || menu.showSessionActions"
        class="mb-4 flex flex-wrap items-center justify-between gap-4"
      >
        <UserMenuCard v-if="menu.showUser" />

        <!-- `autofocus` cubre el primer montaje, que es el único que hay. -->
        <SearchField
          v-if="menu.searchPosition === 'top'"
          ref="searchField"
          v-model="filter"
          :label="t('components.SearchMenuComponent.placeholder')"
          :disabled="isMenuEmpty"
          autofocus
          class="min-w-48 grow" />

        <MenuSessionActions v-if="menu.showSessionActions" :actions="sessionActions" />
      </div>

      <transition enter-active-class="transition-opacity duration-200 ease-ui" leave-active-class="transition-opacity duration-150 ease-ui" enter-from-class="opacity-0" leave-to-class="opacity-0" mode="out-in">
        <div v-if="isMenuEmpty" key="empty-state" class="flex flex-1 min-h-0 items-center justify-center">
          <EmptyState :title="t('views.menu.noApps')" icon="application-x-executable" />
        </div>
        <div v-else-if="filter !== ''" key="filter-view" class="flex-1 min-h-0 overflow-y-auto">
          <FilterArea :apps="appsFiltred" :selected-index="selectedIndex" />
        </div>
        <component :is="layoutComponent" v-else key="main-view" :controller="controller" :menu="menu" />
      </transition>

      <div v-if="menu.searchPosition === 'bottom'" class="mt-4 shrink-0">
        <SearchField
          ref="searchField"
          v-model="filter"
          :label="t('components.SearchMenuComponent.placeholder')"
          :disabled="isMenuEmpty"
          autofocus
          class="w-full" />
      </div>
    </div>
  </AppletPopover>
</template>
