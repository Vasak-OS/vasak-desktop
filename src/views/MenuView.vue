
<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, type Ref, ref } from 'vue';
import FilterArea from '@/components/areas/menu/FilterArea.vue';
import MenuArea from '@/components/areas/menu/MenuArea.vue';
import CategoryMenuPill from '@/components/buttons/CategoryMenuPill.vue';
import SessionButton from '@/components/buttons/SessionButton.vue';
import UserMenuCard from '@/components/cards/UserMenuCard.vue';
import SearchMenuComponent from '@/components/SearchMenuComponent.vue';
import WeatherWidget from '@/components/widgets/WeatherWidget.vue';
import { getMenuItems } from '@/services/app.service';
import {
	detectDisplayServer as sysDetectDisplayServer,
	logout as sysLogout,
	reboot as sysReboot,
	shutdown as sysShutdown,
	suspend as sysSuspend,
} from '@/services/system.service';
import { openConfigurationWindow, toggleMenu } from '@/services/window.service';
import { logError } from '@/utils/logger';

const menuData: Ref<Array<any>> = ref([]);
const categorySelected: Ref<any> = ref('all');
const filter: Ref<string> = ref('');
const logoutImg: Ref<string> = ref('');
const shutdownImg: Ref<string> = ref('');
const rebootImg: Ref<string> = ref('');
const suspendImg: Ref<string> = ref('');
const settingsImg: Ref<string> = ref('');

const setMenu = async () => {
	try {
		menuData.value = await getMenuItems();
	} catch (error) {
		logError('Error al cargar el menú:', error);
	}
};

const detectDisplayServer = async () => {
	try {
		const result = await sysDetectDisplayServer();
		return result;
	} catch (error) {
		logError('Error detectando servidor de display:', error);
		return 'unknown';
	}
};

const logout = async () => {
	try {
		const displayServer = await detectDisplayServer();
		await sysLogout({ displayServer });
	} catch (error) {
		logError('Error al hacer logout:', error);
	}
};

const shutdown = async () => {
	try {
		await sysShutdown();
	} catch (error) {
		logError('Error al apagar:', error);
	}
};

const reboot = async () => {
	try {
		await sysReboot();
	} catch (error) {
		logError('Error al reiniciar:', error);
	}
};

const suspend = async () => {
	try {
		const displayServer = await detectDisplayServer();
		await sysSuspend({ displayServer });
	} catch (error) {
		logError('Error al suspender:', error);
	}
};

const openConfiguration = async () => {
	try {
		await openConfigurationWindow();
	} catch (error) {
		logError('Error al abrir configuración:', error);
	}
};

const apps = computed(() => {
	const allApps = (menuData.value as any).all?.apps;
	return allApps;
});

const appsOfCategory = computed(() => (menuData.value as any)[categorySelected.value]?.apps);

const setImages = async () => {
	try {
		logoutImg.value = await getIconSource('system-log-out');
		shutdownImg.value = await getIconSource('system-shutdown');
		rebootImg.value = await getIconSource('system-reboot');
		suspendImg.value = await getIconSource('system-suspend');
		settingsImg.value = await getIconSource('settings');
	} catch (error) {
		logError('Error loading session icons:', error);
	}
};

onMounted(async () => {
	setMenu();
	setImages();
	document.addEventListener('keydown', (event) => {
		if (event.key === 'Escape') {
			try {
				toggleMenu();
			} catch (error) {
				logError('Error al cerrar menú:', error);
			}
		}
	});
});
</script>

<template>
  <div class="h-screen p-4 rounded-[calc(var(--border-radius)+16px)] background">
    <div
      class="flex items-center justify-between animate-fade-in mb-4 header-section"
    >
      <UserMenuCard />

      <SearchMenuComponent v-model:filter="filter" class="search-component" />

      <div class="flex items-center space-x-2">
        <SessionButton
          v-for="(action, index) in [
            {
              title: 'Configuration',
              img: settingsImg,
              handler: openConfiguration,
            },
            { title: 'Shutdown', img: shutdownImg, handler: shutdown },
            { title: 'Reboot', img: rebootImg, handler: reboot },
            { title: 'Logout', img: logoutImg, handler: logout },
            { title: 'Suspend', img: suspendImg, handler: suspend },
          ]"
          :key="index"
          :title="action.title"
          :img="action.img"
          @click="action.handler"
          class="w-10 h-10 hover:bg-primary/30 rounded-corner p-1 transform transition-all duration-200 ease-out hover:scale-110 hover:rotate-3"
        />
      </div>
    </div>

    <transition enter-active-class="transition-opacity duration-300 ease-out" leave-active-class="transition-opacity duration-300 ease-out" enter-from-class="opacity-0" leave-to-class="opacity-0" mode="out-in">
      <div v-if="filter !== ''" key="filter-view" class="animate-fade-in">
        <FilterArea v-model:apps="apps" v-model:filter="filter" />
      </div>
      <div
        v-else
        key="main-view"
        class="grid grid-cols-3 gap-4 animate-slide-up-plus h-[calc(100vh-88px)]"
      >
        <div
          class="background rounded-corner p-4 h-full overflow-y-auto animate-[fade-in_0.5s_ease-out_0.2s_backwards]"
        >
          <MenuArea v-model:apps="appsOfCategory" />
        </div>

        <div
          class="rounded-corner background p-4 space-y-4 h-full overflow-y-auto animate-[fade-in_0.5s_ease-out_0.2s_backwards]"
        >
          <WeatherWidget />
        </div>

        <div class="animate-[fade-in_0.5s_ease-out_0.2s_backwards]">
          <transition-group
            tag="div"
            move-class="transition-transform duration-400 ease-out" enter-active-class="transition-all duration-400 ease-out" leave-active-class="transition-all duration-400 ease-out" enter-from-class="opacity-0 translate-y-[20px] scale-90" leave-to-class="opacity-0 translate-y-[20px] scale-90"
            appear
            class="flex flex-wrap flex-row justify-center gap-3"
          >
            <CategoryMenuPill
              v-for="(value, key) in menuData"
              :key="key"
              :category="key"
              :image="value.icon"
              :description="value.description"
              v-model:categorySelected="categorySelected"
              class="transition-all duration-200 ease-out hover:-translate-y-1 hover:scale-[1.03] hover:shadow-[0_4px_15px_rgba(0,0,0,0.1)]"
            />
          </transition-group>
        </div>
      </div>
    </transition>
  </div>
</template>

