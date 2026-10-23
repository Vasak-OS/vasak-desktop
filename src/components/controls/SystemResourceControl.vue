<script lang="ts" setup>
/**
 * CPU, memoria y temperatura de un vistazo en el panel (vasak-desktop#216).
 *
 * Los mide el applet de Rust (`system_resources`) cada dos segundos, los mismos
 * contadores que el widget de escritorio; acá no hay temporizador. El uso de CPU
 * sale de dos lecturas, con la misma cuenta que el widget (`system-stats.ts`),
 * así que hay que guardar la anterior —nada más—. Un clic abre `vasak-monitor`,
 * que abre en Recursos, donde se sigue el detalle.
 */
import { Command } from '@tauri-apps/plugin-shell';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ref } from 'vue';
import { useSharedEvent } from '@/tools/event.bus';
import { cpuPercent, memoryPercent, type SystemStats } from '@/tools/system-stats';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const previous = ref<SystemStats | null>(null);
const cpu = ref(0);
const memory = ref(0);
const temperature = ref<number | null>(null);

useSharedEvent<SystemStats>('system-resources', (stats) => {
	cpu.value = cpuPercent(previous.value, stats);
	memory.value = memoryPercent(stats);
	temperature.value = stats.temperature_c;
	previous.value = stats;
});

const openMonitor = async (): Promise<void> => {
	try {
		await Command.create('vasak-monitor').spawn();
	} catch (error) {
		logError('[recursos] no se pudo abrir vasak-monitor:', error);
	}
};
</script>

<template>
  <!-- Hasta la primera medición no hay nada que mostrar: el applet tarda un
       intervalo en tener la primera lectura. -->
  <button
    v-if="previous"
    type="button"
    class="flex flex-col justify-center rounded-corner-m px-1 leading-none cursor-pointer transition-colors duration-200 ease-ui hover:bg-ui-hover active:bg-ui-pressed"
    :title="t('components.SystemResourceControl.title')"
    @click="openMonitor"
  >
    <span class="flex items-center gap-0.5 text-[9px] tabular-nums text-tx-main">
      <span class="text-tx-muted">{{ t('components.SystemResourceControl.cpu') }}</span>{{ cpu }}%
    </span>
    <span class="flex items-center gap-0.5 text-[9px] tabular-nums text-tx-main">
      <span class="text-tx-muted">{{ t('components.SystemResourceControl.memory') }}</span>{{ memory }}%
    </span>
    <!-- La temperatura sólo cuando hay sensor: una VM o una máquina sin zona
         térmica no muestra una fila vacía ni un cero inventado. -->
    <span
      v-if="temperature !== null"
      class="flex items-center gap-0.5 text-[9px] tabular-nums text-tx-main"
    >
      {{ Math.round(temperature) }}°
    </span>
  </button>
</template>
