<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, ref } from 'vue';
import { useSharedEvent } from '@/tools/event.bus';

const { t } = useI18n();

interface NetworkRate {
	/** Bytes por segundo. */
	down: number;
	up: number;
}

const tasa = ref<NetworkRate | null>(null);

// La mide el applet de Rust cada dos segundos leyendo /proc/net/dev; acá no hay
// ni un temporizador ni una cuenta que llevar.
useSharedEvent<NetworkRate>('network-rate', (medida) => {
	tasa.value = medida;
});

/**
 * Una unidad, un decimal y nada más.
 *
 * El ancho del texto es lo que mueve todo lo que tiene al lado en el panel:
 * «1,2 MB/s» y «980 kB/s» ocupan casi lo mismo, mientras que los bytes exactos
 * empujarían los iconos cada dos segundos. Debajo de 1 kB/s se muestra cero:
 * el goteo de fondo de una máquina en reposo no es información.
 */
const legible = (bytes: number): string => {
	if (bytes < 1024) return '0 kB/s';
	if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} kB/s`;

	return `${(bytes / (1024 * 1024)).toFixed(1)} MB/s`;
};

const bajada = computed(() => legible(tasa.value?.down ?? 0));
const subida = computed(() => legible(tasa.value?.up ?? 0));
</script>

<template>
  <!-- Hasta la primera medición no hay nada que mostrar: el applet tarda un
       intervalo en tener dos lecturas que comparar. -->
  <div
    v-if="tasa"
    class="flex flex-col justify-center rounded-corner px-1 leading-none hover:bg-primary"
    :title="t('components.TrayNetworkRateControl.title')"
  >
    <span class="flex items-center gap-0.5 text-[9px] tabular-nums text-tx-main">
      <span aria-hidden="true" class="text-tx-muted">▼</span>{{ bajada }}
    </span>
    <span class="flex items-center gap-0.5 text-[9px] tabular-nums text-tx-main">
      <span aria-hidden="true" class="text-tx-muted">▲</span>{{ subida }}
    </span>
  </div>
</template>
