<script setup lang="ts">
import { ref, onMounted, computed, Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

import SearchMenuComponent from '@/components/SearchMenuComponent.vue';
import MenuArea from '@/components/areas/menu/MenuArea.vue';
import FilterArea from '@/components/areas/menu/FilterArea.vue';
import UserMenuCard from '@/components/cards/UserMenuCard.vue';
import SessionButton from '@/components/buttons/SessionButton.vue';
import CategoryMenuPill from '@/components/buttons/CategoryMenuPill.vue';
import WeatherWidget from '@/components/widgets/WeatherWidget.vue';
import { getIconSource } from '@vasakgroup/plugin-vicons';
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
		menuData.value = await invoke('get_menu_items');
	} catch (error) {
		logError('Error al cargar el menú:', error);
	}
};

const detectDisplayServer = async () => {
	try {
		const result = await invoke('detect_display_server');
		return result;
	} catch (error) {
		logError('Error detectando servidor de display:', error);
		return 'unknown';
	}
};

const logout = async () => {
	try {
		const displayServer = await detectDisplayServer();
		await invoke('logout', { displayServer });
	} catch (error) {
		logError('Error al hacer logout:', error);
	}
};

const shutdown = async () => {
	try {
		await invoke('shutdown');
	} catch (error) {
		logError('Error al apagar:', error);
	}
};

const reboot = async () => {
	try {
		await invoke('reboot');
	} catch (error) {
		logError('Error al reiniciar:', error);
	}
};

const suspend = async () => {
	try {
		const displayServer = await detectDisplayServer();
		await invoke('suspend', { displayServer });
	} catch (error) {
		logError('Error al suspender:', error);
	}
};

const openConfiguration = async () => {
	try {
		await invoke('open_configuration_window');
	} catch (error) {
		logError('Error al abrir configuración:', error);
	}
};

const apps = computed(() => {
	const allApps = (menuData.value as any)['all']?.apps;
	return allApps;
});

const appsOfCategory = computed(
	() => (menuData.value as any)[categorySelected.value]?.apps
);

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
				invoke('toggle_menu');
			} catch (error) {
				console.error('Error al cerrar:', error);
			}
		}
	});
});
</script>

<template>
  <div class="vmenu background">
    <div
      class="flex items-center justify-between animate-fadeIn mb-4 header-section"
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
          class="w-10 h-10 hover:bg-vsk-primary/30 rounded-vsk p-1 transform transition-all duration-200 ease-out hover:scale-110 hover:rotate-3"
        />
      </div>
    </div>

    <transition name="fade" mode="out-in">
      <div v-if="filter !== ''" key="filter-view" class="animate-fadeIn">
        <FilterArea v-model:apps="apps" v-model:filter="filter" />
      </div>
      <div
        v-else
        key="main-view"
        class="grid grid-cols-3 gap-4 animate-slideUpPlus h-[calc(100vh-88px)]"
      >
        <div
          class="background rounded-vsk p-4 h-full overflow-y-auto apps-container"
        >
          <MenuArea v-model:apps="appsOfCategory" />
        </div>

        <div
          class="rounded-vsk background p-4 space-y-4 h-full overflow-y-auto weather-container"
        >
          <WeatherWidget />
        </div>

        <div class="categories-container">
          <transition-group
            tag="div"
            name="list-stagger"
            appear
            class="flex flex-wrap flex-row justify-center category-pills-wrapper"
          >
            <CategoryMenuPill
              v-for="(value, key) in menuData"
              :key="key"
              :category="key"
              :image="value.icon"
              :description="value.description"
              v-model:categorySelected="categorySelected"
              class="category-pill"
            />
          </transition-group>
        </div>
      </div>
    </transition>
  </div>
</template>

<style>
@reference "../style.css";

.vmenu {
  @apply h-screen p-4;
  border-radius: calc(var(--border-radius) + 16px);
}

.header-section {
  animation-duration: 0.5s;
}

.search-component {
  @apply w-2/5 border-b-2 border-vsk-primary;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes slideUpPlus {
  from {
    opacity: 0;
    transform: translateY(30px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.animate-fadeIn {
  animation: fadeIn 0.4s ease-out;
}

.animate-slideUpPlus {
  animation: slideUpPlus 0.5s ease-out forwards;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.list-stagger-enter-active,
.list-stagger-leave-active {
  transition: all 0.4s ease;
}
.list-stagger-enter-from,
.list-stagger-leave-to {
  opacity: 0;
  transform: translateY(20px) scale(0.9);
}
.list-stagger-move {
  transition: transform 0.4s ease;
}

.category-pills-wrapper {
  gap: 0.75rem;
}

.category-pill {
  transition: transform 0.2s ease-out, box-shadow 0.2s ease-out;
}

.category-pill:hover {
  transform: translateY(-4px) scale(1.03);
  box-shadow: 0 4px 15px rgba(0, 0, 0, 0.1);
}

.apps-container,
.weather-container,
.categories-container {
  animation: fadeIn 0.5s ease-out 0.2s backwards;
}
</style>
