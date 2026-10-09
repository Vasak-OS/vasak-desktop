<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { PanelPill } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { toggleApplet } from '@/services/window.service';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { usePanelDensity } from '@/tools/composables/usePanelDensity';
import { clockParts } from '@/tools/panel-clock';
import { showsNames } from '@/tools/panel-density';
import { logError } from '@/utils/logger';

/**
 * El reloj del panel, en su píldora (vasak-desktop#151, rediseño #168).
 *
 * En el panel horizontal la hora va alta a un lado —ocupa el alto de la
 * píldora— y la fecha apilada al otro, en dos renglones: el día de la semana
 * arriba, el número y el mes abajo. Así se lee de un vistazo. Tocarlo abre el
 * tablero de fecha colgado de acá (vasak-desktop#130). En un panel angosto la
 * fecha se pliega (`showsNames`) y queda sólo la hora, con la fecha entera en
 * el globo.
 *
 * De costado la hora va apilada sobre los minutos: «12:34» son cinco
 * caracteres y en una barra de 36 píxeles de ancho no entran en una línea.
 * Dos de dos dígitos sí, y la fecha entera queda en el globo.
 */
const { t, locale } = useI18n();
const { vertical, hasSurface } = usePanelConfig();
const density = usePanelDensity();

const now = ref(new Date());
const parts = computed(() => clockParts(now.value, locale.value));

/**
 * Wakes up on the minute instead of every five seconds.
 *
 * The clock only shows HH:MM, so waking up at the next minute boundary is both
 * accurate and ~12x fewer wakeups than a fixed five-second timer.
 */
let tickTimer: ReturnType<typeof setTimeout> | undefined;

const scheduleNextTick = () => {
	const msUntilNextMinute = 60_000 - (Date.now() % 60_000);
	tickTimer = setTimeout(() => {
		now.value = new Date();
		scheduleNextTick();
	}, msUntilNextMinute);
};

onMounted(() => {
	now.value = new Date();
	scheduleNextTick();
});

onUnmounted(() => {
	if (tickTimer !== undefined) clearTimeout(tickTimer);
});

const opener = ref<unknown>(null);
const { isOpen } = useOpenApplet('date');
const openLabel = computed(() =>
	t('components.PanelClock.open').replace('{0}', () => parts.value.longDate)
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
  <PanelPill
    ref="opener"
    :label="vertical ? parts.hour : ''"
    :caption="vertical ? parts.minute : ''"
    :expanded="isOpen"
    :title="parts.longDate"
    :accessible-label="openLabel"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    :flat="hasSurface"
    aria-haspopup="dialog"
    class="shrink-0"
    data-clock-pill
    @click="openBoard"
  >
    <!-- Arriba y abajo: la hora alta a un lado, la fecha apilada al otro
         (vasak-desktop#168). De costado la hora/minutos siguen por `label` y
         `caption`, que es lo que entra en una columna de 36 píxeles. -->
    <template v-if="!vertical" #default>
      <span class="flex items-center gap-2 px-0.5">
        <span
          class="font-semibold leading-none tabular-nums text-heading-s"
          data-clock-time
        >
          {{ parts.time }}
        </span>
        <span
          v-if="showsNames(density)"
          class="flex min-w-0 flex-col items-start gap-0.5 text-left leading-none"
          data-clock-date
        >
          <span class="max-w-full truncate text-label-xs leading-none text-tx-muted">
            {{ parts.weekday }}
          </span>
          <span class="max-w-full truncate text-label-xs leading-none">
            {{ parts.dayMonth }}
          </span>
        </span>
      </span>
    </template>
  </PanelPill>
</template>
