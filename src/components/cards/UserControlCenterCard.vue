<template>
  <div
    class="bg-ui-bg/80 rounded-corner border border-ui-border p-4 flex items-center gap-4 w-full transition-all duration-300 hover:bg-secondary hover:scale-[1.02] group"
    :class="{
      'opacity-0 translate-y-4': !isLoaded,
      'opacity-100 translate-y-0': isLoaded,
    }"
  >
    <div
      class="relative w-16 h-16 rounded-full transition-all duration-300 group-hover:scale-110 "
    >
      <img
        :src="userInfo.avatar_data"
        :alt="userInfo.full_name"
        class="h-full w-full aspect-square object-cover transition-all duration-300 "
        :class="{
          'opacity-0 scale-90': !isLoaded,
          'opacity-100 scale-100': isLoaded,
        }"
      />

    </div>
    <div class="flex flex-col flex-1 space-y-1">
      <h2
        class="text-lg font-semibold transition-all duration-300 group-hover:text-primary"
      >
        {{ userInfo.full_name }}
      </h2>
      <p
        class="text-sm text-tx-muted transition-all duration-500 group-hover:opacity-90"
      >
        {{ userInfo.username }}
      </p>
    </div>
    <div
      class="text-right space-y-1 transition-all duration-700"
    >
      <div
        class="text-2xl font-medium transition-all duration-300 tabular-nums text-primary"
        :class="{ 'animate-pulse': isTimeUpdating }"
      >
        {{ currentTime }}
      </div>
      <div
        class="text-sm text-tx-muted transition-all duration-300 capitalize"
      >
        {{ currentDate }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { getUserData, type UserInfo } from '@vasakgroup/plugin-user-data';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { onMounted, onUnmounted, ref } from 'vue';
import { logError } from '@/utils/logger';

const { locale } = useI18n();

const userInfo = ref<UserInfo>({
	username: '',
	full_name: '',
	avatar_data: '',
});

const currentTime = ref('');
const currentDate = ref('');
const isTimeUpdating = ref(false);
const isLoaded = ref(false);

const updateDateTime = () => {
	isTimeUpdating.value = true;

	const now = new Date();
	const newTime = now.toLocaleTimeString(locale.value, {
		hour: '2-digit',
		minute: '2-digit',
	});
	const newDate = now.toLocaleDateString(locale.value, {
		weekday: 'long',
		day: 'numeric',
		month: 'long',
	});

	if (currentTime.value !== newTime) {
		currentTime.value = newTime;
	}
	if (currentDate.value !== newDate) {
		currentDate.value = newDate;
	}

	setTimeout(() => {
		isTimeUpdating.value = false;
	}, 200);
};

const getUserInfo = async () => {
	try {
		const info = await getUserData();
		userInfo.value = info as UserInfo;
	} catch (error) {
		logError('Error obteniendo información de usuario:', error);
	}
};

let relojTimeout: ReturnType<typeof setTimeout> | null = null;

/**
 * Despierta en el próximo cambio de minuto, no una vez por segundo.
 *
 * El reloj muestra hora y minuto —`toLocaleTimeString` con `hour` y `minute`, sin
 * segundos— así que 59 de cada 60 despertares no cambiaban un píxel. Y no eran
 * gratis: cada uno construía un `Date`, hacía dos formateos por locale —medidos
 * en 153 µs juntos, o 4,4 segundos de CPU en una sesión de ocho horas—, escribía
 * `isTimeUpdating` disparando reactividad de Vue, y armaba un `setTimeout`
 * anidado. Todo eso dentro del proceso que está siempre encendido.
 *
 * Se agregan 250 ms al borde del minuto para no despertar justo antes por un
 * redondeo del temporizador y tener que volver a esperar.
 */
const programarProximoMinuto = () => {
	const ahora = new Date();
	const faltaParaElMinuto = (60 - ahora.getSeconds()) * 1000 - ahora.getMilliseconds() + 250;

	relojTimeout = globalThis.setTimeout(() => {
		updateDateTime();
		programarProximoMinuto();
	}, faltaParaElMinuto);
};

onMounted(async () => {
	await getUserInfo();
	updateDateTime();

	setTimeout(() => {
		isLoaded.value = true;
	}, 100);

	programarProximoMinuto();
});

onUnmounted(() => {
	if (relojTimeout) {
		clearTimeout(relojTimeout);
		relojTimeout = null;
	}
});
</script>

