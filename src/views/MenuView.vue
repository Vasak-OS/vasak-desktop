<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onBeforeUnmount, onMounted, type Ref, ref, watch } from 'vue';
import FilterArea from '@/components/areas/menu/FilterArea.vue';
import MenuArea from '@/components/areas/menu/MenuArea.vue';
import CategoryMenuPill from '@/components/buttons/CategoryMenuPill.vue';
import SessionButton from '@/components/buttons/SessionButton.vue';
import UserMenuCard from '@/components/cards/UserMenuCard.vue';
import SearchMenuComponent from '@/components/SearchMenuComponent.vue';
import WeatherWidget from '@/components/widgets/WeatherWidget.vue';
import { getMenuItems, openApp } from '@/services/app.service';
import { openSettings, toggleSessionPopup } from '@/services/window.service';
import { useIcons } from '@/tools/composables/useReactiveIcon';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const menuData: Ref<Record<string, any>> = ref({});
const categorySelected: Ref<any> = ref('all');
const filter: Ref<string> = ref('');
const leaving = ref(false);
const selectedIndex = ref(0);
const menuLoadFailed = ref(false);
const menuWindow = getCurrentWindow();
let unlistenFocus: (() => void) | null = null;

const { logoutImg, shutdownImg, rebootImg, suspendImg, settingsImg } = useIcons({
	logoutImg: 'system-log-out',
	shutdownImg: 'system-shutdown',
	rebootImg: 'system-reboot',
	suspendImg: 'system-suspend',
	settingsImg: 'settings',
});

const setMenu = async () => {
	try {
		const data = await getMenuItems();
		if (!data || Object.keys(data).length === 0) {
			menuLoadFailed.value = true;
			menuData.value = {};
			return;
		}
		// Pre-sort all apps within each category alphabetically by name
		for (const key of Object.keys(data)) {
			if (data[key]?.apps) {
				data[key].apps.sort((a: any, b: any) => a.name.localeCompare(b.name));
			}
		}
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

/**
 * Plays the leave animation and hides the window.
 *
 * Hides rather than toggles. It used to call the toggle, and hiding raises a
 * blur, and the blur handler called it again — by then the window was hidden,
 * so the second call *opened* it. Escape appeared to close the menu and
 * immediately bring it back.
 *
 * The guard is the other half: Escape and losing focus can both fire for the
 * same dismissal, and two overlapping animations left the window half faded.
 */
const closeAfterAnimation = () => {
	if (leaving.value) return;
	leaving.value = true;
	setTimeout(() => {
		menuWindow.hide().catch(() => {
			/* already gone */
		});
	}, 200);
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
	// The window is hidden rather than destroyed now, so it is no longer
	// rebuilt — and re-fetched — on every open. The backend watches the
	// application directories and tells us when an app is installed or removed.
	listen('menu-items-changed', () => setMenu()).then((fn) => {
		unlistenMenuChanged = fn;
	});
	document.addEventListener('keydown', onKeydown);
	window.addEventListener('blur', onBlur);
	menuWindow.onFocusChanged(({ payload: focused }) => {
		if (focused) {
			// Shown again after being hidden: the animation state has to be
			// reset or the menu comes back mid-fade and never becomes solid.
			leaving.value = false;
			setTimeout(() => document.getElementById('search')?.focus(), 50);
			return;
		}
		// Losing focus was never handled — only gaining it — so clicking
		// somewhere else left the menu open over whatever you clicked. The DOM
		// `blur` event does not stand in for this: the webview keeps its own
		// focus when another window takes the compositor's.
		closeAfterAnimation();
	}).then(fn => { unlistenFocus = fn; });
});

onBeforeUnmount(() => {
	document.removeEventListener('keydown', onKeydown);
	window.removeEventListener('blur', onBlur);
	unlistenFocus?.();
	unlistenMenuChanged?.();
});

watch(filter, () => {
	selectedIndex.value = 0;
});

watch(appsFiltred, (list) => {
	if (selectedIndex.value >= list.length) {
		selectedIndex.value = Math.max(0, list.length - 1);
	}
});

const onKeydown = (event: KeyboardEvent) => {
	if (event.key === 'Escape') {
		closeAfterAnimation();
		return;
	}

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
			menuWindow.close();
		}
	}
};

const onBlur = () => {
	closeAfterAnimation();
};
</script>

<template>
  <Transition appear enter-active-class="enter-active">
    <div :class="['h-screen p-4 rounded-corner bg-ui-bg/80 border border-ui-border', { 'leave-active': leaving }]">
    <div
      class="flex items-center justify-between gap-4 mb-4 header-section"
    >
      <UserMenuCard />

      <SearchMenuComponent v-model:filter="filter" :disabled="isMenuEmpty" class="search-component" />

      <div class="flex items-center gap-2">
        <SessionButton
          v-for="(action, index) in [
            {
              title: t('views.menu.configuration'),
              img: settingsImg,
              handler: openConfiguration,
            },
            { title: t('views.menu.shutdown'), img: shutdownImg, handler: () => openSessionPopup('shutdown') },
            { title: t('views.menu.reboot'), img: rebootImg, handler: () => openSessionPopup('reboot') },
            { title: t('views.menu.logout'), img: logoutImg, handler: () => openSessionPopup('logout') },
            { title: t('views.menu.suspend'), img: suspendImg, handler: () => openSessionPopup('suspend') },
          ]"
          :key="index"
          :title="action.title"
          :img="action.img"
          @click="action.handler"
          class="w-10 h-10 hover:bg-primary rounded-corner p-1 transform transition-all duration-200 ease-out hover:scale-110 hover:rotate-3"
        />
      </div>
    </div>

    <transition enter-active-class="transition-opacity duration-300 ease-out" leave-active-class="transition-opacity duration-300 ease-out" enter-from-class="opacity-0" leave-to-class="opacity-0" mode="out-in">
      <div v-if="isMenuEmpty" key="empty-state" class="flex items-center justify-center h-[calc(100vh-88px)]">
        <p class="text-tx-main/60 text-lg">{{ t('views.menu.noApps') }}</p>
      </div>
      <div v-else-if="filter !== ''" key="filter-view">
        <FilterArea :apps="appsFiltred" :selected-index="selectedIndex" />
      </div>
      <div
        v-else
        key="main-view"
        class="grid grid-cols-3 gap-4 h-[calc(100vh-88px)]"
      >
        <div
          class="bg-ui-bg/80 border border-ui-border rounded-corner p-4 h-full overflow-y-auto"
        >
          <MenuArea :apps="appsOfCategory" />
        </div>

        <div
          class="col-span-2 grid gap-4 h-full min-h-0"
          style="grid-template-rows: 1fr 2fr"
        >
          <div class="rounded-corner bg-ui-bg/80 border border-ui-border p-4 overflow-y-auto min-h-0">
            <div class="h-full grid gap-3 min-h-0" style="grid-template-columns: 1fr 2fr">
              <div v-if="categoryEntries.all" class="flex items-center justify-center">
                <CategoryMenuPill
                  :category="categoryEntries.all[0]"
                  :image="categoryEntries.all[1].icon"
                  :description="t(categoryEntries.all[1].description)"
                  v-model:categorySelected="categorySelected"
                  large
                  class="w-full h-full"
                />
              </div>

              <transition-group
                tag="div"
                move-class="transition-transform duration-400 ease-out" enter-active-class="transition-all duration-400 ease-out" leave-active-class="transition-all duration-400 ease-out" enter-from-class="opacity-0 translate-y-[20px] scale-90" leave-to-class="opacity-0 translate-y-[20px] scale-90"
                appear
                class="grid grid-cols-3 grid-rows-2 gap-3 min-h-0"
              >
                <CategoryMenuPill
                  v-for="([key, value]) in categoryEntries.others.slice(0, 6)"
                  :key="key"
                  :category="key"
                  :image="value.icon"
                  :description="t(value.description)"
                  v-model:categorySelected="categorySelected"
                />
              </transition-group>
            </div>
          </div>

          <div class="rounded-corner bg-ui-bg/80 border border-ui-border p-4 overflow-y-auto min-h-0">
            <WeatherWidget />
          </div>
        </div>
      </div>
    </transition>
    </div>
  </Transition>
</template>

<style scoped>
@keyframes scale-in {
  from {
    transform: scale(0.95);
    opacity: 0;
  }
  to {
    transform: scale(1);
    opacity: 1;
  }
}

@keyframes scale-out {
  from {
    transform: scale(1);
    opacity: 1;
  }
  to {
    transform: scale(0.95);
    opacity: 0;
  }
}

.enter-active {
  animation: scale-in 200ms ease-out;
}

.leave-active {
  animation: scale-out 200ms ease-in;
}
</style>

