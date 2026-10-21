<script lang="ts" setup>
/**
 * Las estadísticas del sistema en el escritorio (vasak-desktop#269): CPU,
 * memoria, red y temperatura.
 *
 * CPU, memoria y temperatura se leen cada dos segundos del backend
 * (`system_stats`) **sólo mientras el widget está montado**; la red es la tasa
 * que el applet del panel ya emite (`network-rate`), sin una segunda medición.
 * Cuántos mosaicos entran lo decide la caja (`visibleTiles`).
 */
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useBoxSize } from '@/tools/composables/useBoxSize';
import { useSharedEvent } from '@/tools/event.bus';
import {
	cpuPercent,
	formatRate,
	memoryPercent,
	type StatTile,
	type SystemStats,
	visibleTiles,
} from '@/tools/system-stats';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const INTERVAL_MS = 2000;

const box = ref<HTMLElement | null>(null);
const { width, height } = useBoxSize(box);

const stats = ref<SystemStats | null>(null);
const cpu = ref(0);
const rate = ref<{ down: number; up: number } | null>(null);

useSharedEvent<{ down: number; up: number }>('network-rate', (measure) => {
	rate.value = measure;
});

let timer: ReturnType<typeof setInterval> | undefined;

async function refresh(): Promise<void> {
	try {
		const next = await invoke<SystemStats>('system_stats');
		cpu.value = cpuPercent(stats.value, next);
		stats.value = next;
	} catch (error) {
		logError('[widget-sistema] no se pudieron leer las estadísticas:', error);
	}
}

onMounted(() => {
	void refresh();
	timer = setInterval(refresh, INTERVAL_MS);
});
onUnmounted(() => {
	if (timer !== undefined) clearInterval(timer);
});

const tiles = computed(() =>
	visibleTiles(height.value, width.value, stats.value?.temperature_c != null)
);

const memory = computed(() => (stats.value ? memoryPercent(stats.value) : 0));

/** Cada mosaico: su nombre, el número grande y, si lo tiene, el nivel de la barra. */
const content = computed<Record<StatTile, { label: string; value: string; level?: number }>>(
	() => ({
		cpu: { label: t('widgets.system.cpu'), value: `${cpu.value}%`, level: cpu.value },
		memory: { label: t('widgets.system.memory'), value: `${memory.value}%`, level: memory.value },
		network: {
			label: t('widgets.system.network'),
			value: `↓ ${formatRate(rate.value?.down ?? 0)}  ↑ ${formatRate(rate.value?.up ?? 0)}`,
		},
		temperature: {
			label: t('widgets.system.temperature'),
			value: `${Math.round(stats.value?.temperature_c ?? 0)}°C`,
		},
	})
);
</script>

<template>
  <!-- Mosaicos en una grilla de una o dos columnas: la grilla sigue a la caja,
       no a la ventana. -->
  <div
    ref="box"
    class="grid h-full w-full content-center gap-[3cqmin] overflow-hidden p-[4cqmin]"
    :class="width > 0 && width < 240 ? 'grid-cols-1' : 'grid-cols-2'"
    data-system-stats
  >
    <div
      v-for="tile in tiles"
      :key="tile"
      class="flex min-w-0 flex-col justify-center gap-[1cqmin]"
      :data-stat="tile"
    >
      <span class="truncate text-[clamp(0.6rem,6cqmin,0.8rem)] text-tx-muted">{{ content[tile].label }}</span>
      <span class="truncate font-semibold tabular-nums text-tx-main text-[clamp(0.75rem,9cqmin,1.25rem)]">{{ content[tile].value }}</span>
      <div
        v-if="content[tile].level !== undefined"
        class="h-[1.4cqmin] min-h-1 w-full overflow-hidden rounded-corner-full bg-ui-line"
        role="presentation"
      >
        <div class="h-full rounded-corner-full bg-primary" :style="{ width: `${content[tile].level}%` }"></div>
      </div>
    </div>
  </div>
</template>
