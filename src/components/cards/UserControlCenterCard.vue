<template>
  <div class="@container w-full min-w-0">
    <!-- Sin reacción al pasar el mouse: la tarjeta no se puede tocar. Cambiaba de
         fondo, se agrandaba y le pintaba el nombre de otro color a algo que no
         hace nada al hacerle clic, así que prometía un botón que no existe. Es la
         misma decisión que en los iconos de la bandeja: que no se resalte lo que
         no se puede tocar. La cara es `Avatar` de la librería (2.2.0): la foto
         recortada en círculo, o las iniciales si no hay foto o no carga. -->
    <!-- Un contenedor. En la ventana del centro (350 px → ~326 px de caja) va
         apilado: la foto y el nombre arriba, la hora y la fecha debajo. El modo
         fila —hora a la derecha— recién entra cuando hay ancho de verdad para
         que el nombre y la fecha completa quepan en un renglón (~26 rem); por
         debajo de eso se apila, que es lo que evita que un nombre caiga a dos
         líneas apretado contra la fecha. -->
    <div
      class="flex w-full min-w-0 flex-col gap-3 rounded-corner-l border border-ui-line bg-ui-surface/70 p-4 transition-[opacity,translate] duration-300 ease-ui-out @[26rem]:flex-row @[26rem]:items-center @[26rem]:gap-4"
      :class="{
        'opacity-0 translate-y-4': !isLoaded,
        'opacity-100 translate-y-0': isLoaded,
      }"
      data-user-card
    >
      <div class="flex min-w-0 flex-1 items-center gap-4">
        <Avatar :src="userInfo.avatar_data || null" :name="userInfo.full_name" size="xl" />
        <!-- El nombre en una sola línea: si no entra se recorta con «…» (el
             nombre entero queda en el globo), en vez de partirse en dos
             renglones. Apilado tiene ancho de sobra para la mayoría. -->
        <div class="flex min-w-0 flex-1 flex-col gap-1">
          <h2 class="truncate text-lg font-semibold" :title="userInfo.full_name">
            {{ userInfo.full_name }}
          </h2>
          <p class="truncate text-label-m text-tx-muted">
            {{ userInfo.username }}
          </p>
        </div>
      </div>
      <!-- La hora y la fecha van en la misma fila —la hora grande, la fecha a su
           derecha— y no apiladas (bug #206). Si no entran al lado, la fecha baja
           con gracia (`flex-wrap`). En modo fila de la tarjeta se empujan al
           borde derecho. -->
      <div
        class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 @[26rem]:justify-end"
        data-user-clock
      >
        <div
          class="text-2xl font-medium tabular-nums text-primary"
          :class="{ 'animate-pulse': isTimeUpdating }"
        >
          {{ currentTime }}
        </div>
        <!-- La fecha es lo único que se toca de la tarjeta: abre el calendario
             del mes dentro del centro (vasak-desktop#183). Es el botón fantasma
             de la librería, con el icono del calendario para que se note que
             responde; la tarjeta sigue sin reaccionar. -->
        <ActionButton
          :label="currentDate"
          icon="x-office-calendar"
          icon-type="symbol"
          icon-right
          variant="ghost"
          size="sm"
          custom-class="-me-2"
          :aria-label="openLabel"
          :title="openLabel"
          :aria-expanded="calendarOpen"
          data-user-date
          @click="emit('open-calendar')"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { getUserData, type UserInfo } from '@vasakgroup/plugin-user-data';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton, Avatar } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { capitalizeFirst } from '@/tools/text-case';
import { logError } from '@/utils/logger';

withDefaults(
	defineProps<{ /** Si el calendario del mes está abierto. */ calendarOpen?: boolean }>(),
	{
		calendarOpen: false,
	}
);

const emit = defineEmits<{ /** Se tocó la fecha. */ 'open-calendar': [] }>();

const { t, locale } = useI18n();

const userInfo = ref<UserInfo>({
	username: '',
	full_name: '',
	avatar_data: '',
});

const currentTime = ref('');
const currentDate = ref('');
const isTimeUpdating = ref(false);
const isLoaded = ref(false);

const openLabel = computed(() =>
	t('views.controlCenter.openCalendar').replace('{0}', currentDate.value)
);

const updateDateTime = () => {
	isTimeUpdating.value = true;

	const now = new Date();
	const newTime = now.toLocaleTimeString(locale.value, {
		hour: '2-digit',
		minute: '2-digit',
	});
	// Sólo la primera letra en mayúscula: el `capitalize` de CSS que había la
	// ponía en cada palabra («Martes, 6 De Octubre»).
	const newDate = capitalizeFirst(
		now.toLocaleDateString(locale.value, {
			weekday: 'long',
			day: 'numeric',
			month: 'long',
		}),
		locale.value
	);

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

let clockTimeout: ReturnType<typeof setTimeout> | null = null;

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
const scheduleNextMinute = () => {
	const now = new Date();
	const untilNextMinute = (60 - now.getSeconds()) * 1000 - now.getMilliseconds() + 250;

	clockTimeout = globalThis.setTimeout(() => {
		updateDateTime();
		scheduleNextMinute();
	}, untilNextMinute);
};

onMounted(async () => {
	await getUserInfo();
	updateDateTime();

	setTimeout(() => {
		isLoaded.value = true;
	}, 100);

	scheduleNextMinute();
});

onUnmounted(() => {
	if (clockTimeout) {
		clearTimeout(clockTimeout);
		clockTimeout = null;
	}
});
</script>

