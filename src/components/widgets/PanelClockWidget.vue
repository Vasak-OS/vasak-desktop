<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { toggleApplet } from '@/services/window.service';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { logError } from '@/utils/logger';

const { t } = useI18n();

interface TimeData {
	day: string;
	month: string;
	year: string;
	hour: string;
	minute: string;
}

/**
 * De costado la hora va apilada sobre los minutos.
 *
 * «12:34» son cinco caracteres: en una barra de 36 píxeles de ancho no entran
 * en una línea, y encogerlos hasta que entren los deja ilegibles. Dos líneas de
 * dos dígitos sí entran, y el día completo sigue estando en el `title`.
 */
const { vertical } = usePanelConfig();

const timeData = ref<TimeData>({
	day: '00',
	month: '00',
	year: '0000',
	hour: '00',
	minute: '00',
});

const formatNumber = (num: number): string => num.toString().padStart(2, '0');

const updateTime = () => {
	const date = new Date();
	timeData.value = {
		hour: formatNumber(date.getHours()),
		minute: formatNumber(date.getMinutes()),
		day: formatNumber(date.getDate()),
		month: formatNumber(date.getMonth() + 1),
		year: date.getFullYear().toString(),
	};
};

/**
 * Wakes up on the minute instead of every five seconds.
 *
 * The old timer fired 12 times a minute for a clock that only shows HH:MM, and
 * because it was not aligned to the minute the displayed time could be up to
 * five seconds stale. Scheduling to the next minute boundary is both accurate
 * and ~12x fewer wakeups. The interval was also never cleared.
 */
let tickTimer: ReturnType<typeof setTimeout> | undefined;

const scheduleNextTick = () => {
	const msUntilNextMinute = 60_000 - (Date.now() % 60_000);
	tickTimer = setTimeout(() => {
		updateTime();
		scheduleNextTick();
	}, msUntilNextMinute);
};

onMounted(() => {
	updateTime();
	scheduleNextTick();
});

onUnmounted(() => {
	if (tickTimer !== undefined) clearTimeout(tickTimer);
});

/**
 * El tablero de fecha, colgado del reloj (vasak-desktop#130).
 *
 * El reloj es el lugar donde se busca «qué día es y qué tengo hoy», y hasta
 * acá no abría nada. Ahora es un botón: abre el tablero debajo de él y se
 * realza mientras está abierto, como los demás del panel.
 */
const opener = ref<HTMLElement | null>(null);
const { isOpen, openClasses } = useOpenApplet('date');
const fullDate = computed(
	() => `${timeData.value.day}/${timeData.value.month}/${timeData.value.year}`
);
const openLabel = computed(() =>
	t('components.PanelClock.open').replace('{0}', () => fullDate.value)
);

async function openBoard(): Promise<void> {
	try {
		await toggleApplet('date', opener.value);
	} catch (error) {
		logError('[PanelClock] no se pudo abrir el tablero de fecha:', error);
	}
}
</script>

<template>
  <button
    ref="opener"
    type="button"
    class="flex items-center justify-center rounded-corner-m p-1 font-mono transition-colors duration-200 ease-ui hover:bg-ui-hover active:bg-ui-pressed focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ui-focus"
    :class="[vertical ? 'flex-col text-label-xs leading-tight' : 'text-label-m', openClasses]"
    :title="fullDate"
    :aria-label="openLabel"
    :aria-expanded="isOpen"
    aria-haspopup="dialog"
    @click="openBoard"
  >
    <span aria-hidden="true">
      <template v-if="vertical">{{ timeData.hour }}</template>
      <template v-else>{{ timeData.hour }}:{{ timeData.minute }}</template>
    </span>
    <span v-if="vertical" aria-hidden="true">
      {{ timeData.minute }}
    </span>
  </button>
</template>

