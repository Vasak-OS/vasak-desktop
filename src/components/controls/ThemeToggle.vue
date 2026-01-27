<template>
  <div class="relative inline-block theme-toggle-wrapper" :class="{ 'theme-switching': isSwitching }">
    <!-- Sun/Moon indicator -->
    <div
      class="absolute top-1 right-1 w-3 h-3 rounded-full transition-all duration-500 z-20"
      :class="{
        'bg-yellow-400 animate-pulse': !(configStore?.config as any)?.style?.darkmode,
        'bg-blue-400 animate-pulse': (configStore?.config as any)?.style?.darkmode,
      }"
    ></div>

    <!-- Background gradient effect -->
    <div
      class="absolute inset-0 rounded-vsk transition-all duration-500 pointer-events-none opacity-0 group-hover:opacity-100"
      :class="{
        'bg-lineal-to-br from-orange-400/20 to-yellow-400/20':
          !(configStore?.config as any)?.style?.darkmode,
        'bg-lineal-to-br from-purple-500/20 to-blue-600/20':
          (configStore?.config as any)?.style?.darkmode,
      }"
    ></div>

    <ToggleControl
      :icon="icon"
      :alt="
        (configStore?.config as any)?.style?.darkmode
          ? 'Toggle light theme'
          : 'Toggle dark theme'
      "
      :tooltip="
        (configStore?.config as any)?.style?.darkmode
          ? 'Toggle light theme'
          : 'Toggle dark theme'
      "
      :is-active="true"
      :is-loading="isSwitching"
      :custom-class="{
        'h-[70px] w-[70px] p-2': true,
        'ring-2 ring-vsk-primary/50': true,
      }"
      :icon-class="{
        'w-[50px] h-[50px]': true,
        'filter brightness-110': !(configStore?.config as any)?.style?.darkmode,
      }"
      @click="toggleTheme"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, Ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useConfigStore, setDarkMode } from '@vasakgroup/plugin-config-manager';
import { ToggleControl } from '@vasakgroup/vue-libvasak';
import dark from '@/assets/img/dark.png';
import light from '@/assets/img/light.png';
import { Store } from 'pinia';

const configStore = ref<any>(null);
const isSwitching: Ref<boolean> = ref(false);

onMounted(() => {
	configStore.value = useConfigStore() as Store<'config', { config: any; loadConfig: () => Promise<void>; }>;; 
});

const icon = computed(() => {
	return configStore.value?.config?.style?.darkmode ? light : dark;
});

const toggleTheme = async () => {
	if (isSwitching.value || !configStore.value) return;

	isSwitching.value = true;
	try {
		await invoke('toggle_system_theme');
		await setDarkMode(!(configStore.value.config as any).style.darkmode || false);
	} catch (error) {
		console.error('Error toggling system theme:', error);
	} finally {
		setTimeout(() => {
			isSwitching.value = false;
		}, 800);
	}
};
</script>

<style scoped>
/* Efecto especial para el cambio de tema - aplicado al wrapper */
.theme-toggle-wrapper.theme-switching :deep(button) {
  animation: themeTransition 0.8s ease-in-out;
}

@keyframes themeTransition {
  0% {
    transform: scale(1) rotate(0deg);
  }
  25% {
    transform: scale(1.1) rotate(90deg);
  }
  50% {
    transform: scale(1.15) rotate(180deg);
  }
  75% {
    transform: scale(1.1) rotate(270deg);
  }
  100% {
    transform: scale(1) rotate(360deg);
  }
}

/* Efecto de hover en el fondo */
.theme-toggle-wrapper :deep(button:hover) ~ .absolute.inset-0.rounded-vsk {
  opacity: 1 !important;
}

.theme-toggle-wrapper:hover .absolute.inset-0.rounded-vsk {
  opacity: 1 !important;
}

/* Efecto de hover en la imagen */
.theme-toggle-wrapper :deep(button:hover img) {
  filter: brightness(1.2) drop-shadow(0 4px 8px rgba(0, 0, 0, 0.3));
}
</style>
