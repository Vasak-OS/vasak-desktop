<template>
  <!-- Sin reacción al pasar el mouse: la tarjeta no se puede tocar. Cambiaba de
       fondo, se agrandaba y le pintaba el nombre de otro color a algo que no
       hace nada al hacerle clic, así que prometía un botón que no existe. Es la
       misma decisión que en los iconos de la bandeja: que no se resalte lo que
       no se puede tocar. La cara es `Avatar` de la librería (2.2.0): la foto
       recortada en círculo, o las iniciales si no hay foto o no carga. -->
  <div
    class="flex w-full min-w-0 items-center gap-4 rounded-corner-l border border-ui-line bg-ui-surface/70 p-4 transition-[opacity,translate] duration-300 ease-ui-out"
    :class="{
      'opacity-0 translate-y-4': !isLoaded,
      'opacity-100 translate-y-0': isLoaded,
    }"
  >
    <Avatar :src="userInfo.avatar_data || null" :name="userInfo.full_name" size="xl" />
    <!-- El nombre se parte entre palabras, nunca adentro de una: la columna
         no baja de su palabra más larga, y la de la hora se acomoda. -->
    <div class="flex flex-1 flex-col gap-1">
      <h2 class="text-lg font-semibold">
        {{ userInfo.full_name }}
      </h2>
      <p class="text-label-m text-tx-muted">
        {{ userInfo.username }}
      </p>
    </div>
    <div class="min-w-0 space-y-1 text-right">
      <div
        class="text-2xl font-medium tabular-nums text-primary"
        :class="{ 'animate-pulse': isTimeUpdating }"
      >
        {{ currentTime }}
      </div>
      <div class="text-label-m text-tx-muted capitalize">
        {{ currentDate }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { getUserData, type UserInfo } from '@vasakgroup/plugin-user-data';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { Avatar } from '@vasakgroup/vue-libvasak';
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

