<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */
/** biome-ignore-all lint/correctness/noUnusedVariables: usados en la plantilla */
import { convertFileSrc } from '@tauri-apps/api/core';
import { getUserData, type UserInfo } from '@vasakgroup/plugin-user-data';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import WeatherIcon from '@/components/icon/WeatherIcon.vue';
import { useWeather } from '@/tools/composables/useWeather';
import type { MenuConfig } from '@/tools/menu-config';
import { greetingKey } from '@/tools/menu-greeting';
import { logError } from '@/utils/logger';

/**
 * El encabezado hero del menú (vasak-desktop#203): una imagen opcional de fondo,
 * el saludo del día con el nombre de la persona y el clima de ahora.
 *
 * Va sobre el escritorio (dentro del menú, que ya desenfoca Wayfire), así que la
 * superficie es translúcida y **sin `backdrop-blur`**. La imagen va con la
 * opacidad que elige `headerStrength`; el saludo sale de la franja horaria
 * (`menu-greeting.ts`, reutilizando i18n) y el clima del composable `useWeather`,
 * compartido con el resto del escritorio.
 */
const props = defineProps<{ menu: MenuConfig }>();

const { t } = useI18n();
const { current, dayOrNight, weather } = useWeather();

const userInfo = ref<UserInfo | null>(null);

const greeting = computed(() => t(greetingKey(new Date().getHours())));

const displayName = computed(() => userInfo.value?.full_name || userInfo.value?.username || '');

const imageSrc = computed(() =>
	props.menu.headerImage ? convertFileSrc(props.menu.headerImage) : ''
);

/** La opacidad de la imagen, de 0 a 1, desde `headerStrength` (0–100). */
const imageOpacity = computed(() => props.menu.headerStrength / 100);

const temperature = computed(() => {
	const value = current.value?.temperature_2m;
	if (value === undefined || value === null) return '';
	const unit = (weather.value as any)?.current_units?.temperature_2m ?? '';
	return `${value}${unit}`;
});

onMounted(async () => {
	try {
		userInfo.value = await getUserData();
	} catch (error) {
		logError('No se pudo cargar el usuario para el hero:', error);
	}
});
</script>

<template>
  <section
    class="relative mb-4 min-h-24 shrink-0 overflow-hidden rounded-corner-l border border-ui-line bg-ui-surface/70"
  >
    <!-- La imagen va detrás del contenido, con su opacidad, y nunca roba clics. -->
    <img
      v-if="imageSrc"
      :src="imageSrc"
      alt=""
      aria-hidden="true"
      class="pointer-events-none absolute inset-0 h-full w-full object-cover"
      :style="{ opacity: imageOpacity }"
    />

    <div class="relative flex items-center justify-between gap-4 p-4">
      <div v-if="menu.showGreeting" class="min-w-0">
        <p class="text-label-xs text-tx-muted">{{ greeting }}</p>
        <p class="truncate text-heading-s text-tx-main">{{ displayName }}</p>
      </div>

      <div
        v-if="menu.showWeather && current"
        class="flex shrink-0 items-center gap-2 text-tx-main"
      >
        <WeatherIcon :code="current.weather_code" :day-or-night="dayOrNight" size-class="h-8 w-8" />
        <span class="text-heading-s">{{ temperature }}</span>
      </div>
    </div>
  </section>
</template>
