<template>
  <ToggleControl
    v-if="available"
    class="theme-transition"
    :name="enabled ? 'night-light-symbolic' : 'night-light-disabled-symbolic'"
    type="symbol"
    :label="t('components.NightLightToggle.label')"
    :pressed="enabled"
    :is-active="enabled"
    :is-loading="busy"
    :indicator="indicator"
    data-night-light-toggle
    @click="toggle"
  />
</template>

<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
/**
 * «Luz nocturna» como botón redondo, en la fila del estado A del centro de
 * control (vasak-desktop#178). Es el mismo estado que el mosaico de B. Sin
 * `wlsunset` instalado no aparece: en la fila no hay lugar para un botón que
 * no hace nada, y el mosaico de B es el que explica que no está disponible.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ToggleControl, type ToggleIndicator } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import { useNightLight } from '@/tools/composables/useNightLight';

const { t } = useI18n();
const { available, enabled, busy, toggle } = useNightLight();

/** El punto de la esquina, con el acento del esquema cuando está encendida. */
const indicator = computed<ToggleIndicator | null>(() =>
	enabled.value ? { tone: 'accent' } : null
);
</script>
