<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
/**
 * El brillo del estado B del centro de control: un deslizador por monitor, con
 * su nombre (vasak-desktop#189). Con un solo monitor queda un control, igual
 * que en el estado A, sin nombre que lo acompañe.
 *
 * Mover uno escribe sólo ese monitor, y al soltar. Los monitores externos que
 * no se pueden regular —falta ddcutil, el módulo i2c-dev o el permiso, o el
 * monitor no contesta DDC/CI— se ven «no disponible» con el motivo, en vez de
 * desaparecer. El estado sale de `useDisplayBrightness`, el mismo del estado A.
 */
import type { MonitorBrightness } from '@vasakgroup/plugin-display-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { SliderControl, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import { brightnessIcon, brightnessPercentageClass } from '@/tools/brightness-look';
import { useDisplayBrightness } from '@/tools/composables/useDisplayBrightness';
import { ddcNotices, monitorKey, monitorName } from '@/tools/display-brightness';

const { t } = useI18n();
const { monitors, ddc, loaded, percentOf, preview, commit } = useDisplayBrightness();

const format = (key: string, args: readonly string[]) =>
	args.reduce((text, arg, i) => text.replace(`{${i}}`, arg), t(key));

const notices = computed(() =>
	(ddc.value ? ddcNotices(ddc.value) : []).map((notice) => ({
		...notice,
		text: format(notice.key, notice.args),
	}))
);

/** Con un solo monitor, y nada más que nombrar, el deslizador va sin nombre. */
const named = computed(
	() => monitors.value.length > 1 || notices.value.some((notice) => notice.unavailable)
);

const rows = computed(() =>
	monitors.value.map((monitor) => {
		const name = monitorName(monitor, monitors.value, t('components.MonitorBrightness.builtIn'));
		const percent = percentOf(monitor);
		return {
			monitor,
			key: monitorKey(monitor),
			name,
			percent,
			label: format('components.MonitorBrightness.brightnessOf', [name, String(percent)]),
		};
	})
);

/** El `change` nativo del deslizador sube hasta la fila: es al soltar. */
function onChange(monitor: MonitorBrightness, event: Event): void {
	const target = event.target as HTMLInputElement | null;
	if (target?.type !== 'range') return;
	void commit(monitor, Number(target.value));
}
</script>

<template>
  <section
    class="flex w-full min-w-0 flex-col gap-2"
    :aria-label="t('components.MonitorBrightness.title')"
    data-monitor-brightness
  >
    <div
      v-for="row in rows"
      :key="row.key"
      class="flex min-w-0 flex-col gap-1"
      :data-monitor="row.key"
      @change="onChange(row.monitor, $event)"
    >
      <span
        v-if="named"
        class="truncate px-1 text-label-s text-tx-muted"
        data-monitor-name
      >{{ row.name }}</span>
      <SliderControl
        :name="brightnessIcon(row.percent)"
        type="symbol"
        :label="row.label"
        :model-value="row.percent"
        :min="0"
        :max="100"
        :show-button="false"
        :get-percentage-class="brightnessPercentageClass"
        @update:model-value="preview(row.monitor, $event)"
      />
    </div>

    <!-- Nada que regular: ni panel interno ni monitores externos que contesten. -->
    <p
      v-if="loaded && rows.length === 0 && !notices.some((notice) => notice.unavailable)"
      class="flex min-w-0 items-center gap-2 rounded-corner-l border border-ui-line bg-ui-surface/70 p-3 text-body-s text-tx-muted"
      data-unavailable="true"
    >
      <ThemeIcon name="display-brightness-symbolic" type="symbol" :size="16" alt="" />
      <span class="min-w-0 break-words">{{ t('components.MonitorBrightness.none') }}</span>
    </p>

    <p
      v-for="notice in notices"
      :key="notice.text"
      :class="[
        'flex min-w-0 items-center gap-2 px-1 text-body-s text-tx-muted',
        notice.unavailable ? 'rounded-corner-l border border-ui-line bg-ui-surface/70 p-3' : '',
      ]"
      :data-unavailable="notice.unavailable ? 'true' : undefined"
      :data-output="notice.args[0]"
      data-ddc-notice
      aria-live="polite"
    >
      <ThemeIcon
        v-if="notice.unavailable"
        name="display-brightness-symbolic"
        type="symbol"
        :size="16"
        alt=""
      />
      <span class="min-w-0 break-words">{{ notice.text }}</span>
    </p>
  </section>
</template>
