<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
/**
 * El perfil de energía en el estado B del centro de control: Ahorro ·
 * Equilibrado · Rendimiento, con el `SegmentedControl` de la librería
 * (vasak-desktop#189).
 *
 * Las opciones son las que ofrece el equipo, en el orden del demonio. Sin
 * power-profiles-daemon se ven las tres deshabilitadas y «No disponible»: el
 * control está, pero no hace nada. Si el demonio limitó el rendimiento (por
 * temperatura, o la notebook sobre las piernas) se dice.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { SegmentedControl, type SegmentedOption, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import { usePowerProfile } from '@/tools/composables/usePowerProfile';
import { DEFAULT_PROFILES, profileIcon, profileLabelKey } from '@/tools/power-profile';

const { t } = useI18n();
const { state, loaded, choose } = usePowerProfile();

const available = computed(() => state.value.available);

const options = computed<SegmentedOption<string>[]>(() => {
	const profiles: readonly string[] = available.value ? state.value.profiles : DEFAULT_PROFILES;
	return profiles.map((profile) => {
		const key = profileLabelKey(profile);
		return {
			value: profile,
			label: key ? t(key) : profile,
			disabled: !available.value,
		};
	});
});

const status = computed(() => {
	if (!loaded.value) return '';
	if (!available.value) return t('components.PowerProfileControl.unavailable');
	if (state.value.performanceDegraded) return t('components.PowerProfileControl.degraded');
	return '';
});
</script>

<template>
  <section
    class="flex w-full min-w-0 flex-col gap-2 rounded-corner-l border border-ui-line bg-ui-surface/70 p-3 text-tx-main"
    :data-unavailable="available ? undefined : 'true'"
    data-power-profile
  >
    <!-- En un centro angosto el estado pasa abajo del título en vez de cortarlo. -->
    <div class="flex min-w-0 flex-wrap items-center gap-2">
      <ThemeIcon :name="profileIcon(state.activeProfile)" type="symbol" :size="16" alt="" />
      <span class="min-w-0 flex-1 break-words text-label-m">{{ t('components.PowerProfileControl.title') }}</span>
      <span
        v-if="status"
        class="min-w-0 break-words text-label-s text-tx-muted"
        data-power-status
      >{{ status }}</span>
    </div>
    <SegmentedControl
      :model-value="state.activeProfile"
      :options="options"
      :label="t('components.PowerProfileControl.title')"
      :disabled="!available"
      class="w-full"
      @change="choose"
    />
  </section>
</template>
