<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
/**
 * El brillo del estado A del centro de control: un solo deslizador, el del
 * monitor principal —el panel interno si hay uno, si no el primer monitor
 * externo— (vasak-desktop#189). La lista con uno por monitor está en el estado
 * B (`MonitorBrightnessList`).
 *
 * Sin pantalla que se pueda atenuar no ocupa lugar: no hay nada que regular.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { SliderControl } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import { brightnessIcon, brightnessPercentageClass } from '@/tools/brightness-look';
import { useDisplayBrightness } from '@/tools/composables/useDisplayBrightness';

const { t } = useI18n();
const { primary, percentOf, preview, commit } = useDisplayBrightness();

const value = computed(() => (primary.value ? percentOf(primary.value) : 0));

const label = computed(() =>
	t('components.BrightnessControl.brightness').replace('{0}', String(value.value))
);

function onInput(percent: number): void {
	if (primary.value) preview(primary.value, percent);
}

/** El `change` nativo del deslizador sube hasta acá: es al soltar. */
function onChange(event: Event): void {
	const target = event.target as HTMLInputElement | null;
	if (!primary.value || target?.type !== 'range') return;
	void commit(primary.value, Number(target.value));
}
</script>

<template>
  <SliderControl
    v-if="primary"
    :name="brightnessIcon(value)"
    type="symbol"
    :label="label"
    :model-value="value"
    :min="0"
    :max="100"
    :show-button="false"
    :get-percentage-class="brightnessPercentageClass"
    data-primary-brightness
    @update:model-value="onInput"
    @change="onChange"
  />
</template>
