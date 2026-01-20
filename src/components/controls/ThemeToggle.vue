<template>
  <button
    @click="toggleTheme"
    class="p-2 rounded-vsk background hover:opacity-50 transition-all duration-500 h-[70px] w-[70px] group relative overflow-hidden hover:scale-105 hover:shadow-lg active:scale-95 ring-2 ring-vsk-primary/50"
    :class="{ 'theme-switching': isSwitching }"
  >
    <!-- Background gradient effect -->
    <div
      class="absolute inset-0 rounded-vsk transition-all duration-500"
      :class="{
        'bg-gradient-to-br from-orange-400/20 to-yellow-400/20':
          !(configStore?.config as any)?.style?.darkmode,
        'bg-gradient-to-br from-purple-500/20 to-blue-600/20':
          (configStore?.config as any)?.style?.darkmode,
      }"
      style="opacity: 0"
    ></div>

    <!-- Sun/Moon indicator -->
    <div
      class="absolute top-1 right-1 w-3 h-3 rounded-full transition-all duration-500"
      :class="{
        'bg-yellow-400 animate-pulse': !(configStore?.config as any)?.style?.darkmode,
        'bg-blue-400 animate-pulse': (configStore?.config as any)?.style?.darkmode,
      }"
    ></div>

    <!-- Animated rays for sun (light mode) -->
    <div
      v-if="configStore && !(configStore.config as any).style.darkmode"
      class="absolute inset-0 flex items-center justify-center"
    >
      <div
        class="absolute w-12 h-12 animate-spin"
        style="animation-duration: 8s"
      >
        <div
          v-for="i in 8"
          :key="i"
          class="absolute w-0.5 h-2 bg-yellow-400/40 rounded-full"
          :style="{
            transform: `rotate(${i * 45}deg) translateY(-20px)`,
            'transform-origin': 'center 20px',
          }"
        ></div>
      </div>
    </div>

    <!-- Twinkling stars for moon (dark mode) -->
    <div v-if="configStore && (configStore.config as any).style.darkmode" class="absolute inset-0">
      <div
        v-for="i in 6"
        :key="i"
        class="absolute w-1 h-1 bg-blue-300 rounded-full animate-pulse"
        :style="{
          top: `${10 + i * 8}%`,
          left: `${15 + i * 12}%`,
          'animation-delay': `${i * 0.3}s`,
          'animation-duration': '2s',
        }"
      ></div>
    </div>

    <img
      :src="icon"
      :alt="
        (configStore?.config as any)?.style?.darkmode
          ? 'Toggle light theme'
          : 'Toggle dark theme'
      "
      :title="
        (configStore?.config as any)?.style?.darkmode
          ? 'Toggle light theme'
          : 'Toggle dark theme'
      "
      class="m-auto w-[50px] h-[50px] transition-all duration-500 group-hover:scale-110 relative z-10"
      :class="{
        'animate-spin': isSwitching,
        'drop-shadow-lg group-hover:drop-shadow-xl': true,
        'filter brightness-110': !(configStore?.config as any)?.style?.darkmode,
      }"
    />
  </button>
</template>

<script setup lang="ts">
import { ref, computed, Ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useConfigStore, setDarkMode } from '@vasakgroup/plugin-config-manager';
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
/* Efecto especial para el cambio de tema */
.theme-switching {
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
.group:hover .absolute.inset-0.rounded-vsk {
  opacity: 1 !important;
}

/* Animación de parpadeo para las estrellas */
@keyframes twinkle {
  0%,
  100% {
    opacity: 0.3;
    transform: scale(0.8);
  }
  50% {
    opacity: 1;
    transform: scale(1.2);
  }
}

/* Efecto de rayos del sol */
@keyframes sunRays {
  0% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(360deg);
  }
}

.group:hover img {
  filter: brightness(1.2) drop-shadow(0 4px 8px rgba(0, 0, 0, 0.3));
}
</style>
