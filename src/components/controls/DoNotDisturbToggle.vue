<template>
  <ToggleControl
    class="theme-transition"
    name="notifications-disabled-symbolic"
    type="symbol"
    :label="t('components.DoNotDisturbToggle.label')"
    :pressed="enabled"
    :is-active="enabled"
    :is-loading="busy"
    :indicator="indicator"
    @click="toggle"
  />
</template>

<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
/**
 * «No molestar» como botón redondo, en la fila del estado A del centro de
 * control (vasak-desktop#177). Es el mismo estado que el mosaico de B.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ToggleControl, type ToggleIndicator } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import { useDoNotDisturb } from '@/tools/composables/useDoNotDisturb';

const { t } = useI18n();
const { enabled, busy, toggle } = useDoNotDisturb();

/** El punto de la esquina, con el acento del esquema cuando está puesto. */
const indicator = computed<ToggleIndicator | null>(() =>
	enabled.value ? { tone: 'accent' } : null
);
</script>
