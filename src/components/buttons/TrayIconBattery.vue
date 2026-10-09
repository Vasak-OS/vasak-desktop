<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { PanelPill } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import type { BatteryInfo } from '@/interfaces/battery';
import { getBatteryInfo } from '@/services/core.service';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { usePanelDensity } from '@/tools/composables/usePanelDensity';
import { useSharedEvent } from '@/tools/event.bus';
import { showsNumbers } from '@/tools/panel-density';
import { logError } from '@/utils/logger';

const { t } = useI18n();
const { vertical, hasSurface } = usePanelConfig();
const density = usePanelDensity();

const batteryInfo = ref<BatteryInfo>({
	has_battery: false,
	percentage: 0,
	state: 'Unknown',
	is_present: false,
	is_charging: false,
});

const batteryAltText = computed(() => {
	if (!batteryInfo.value.has_battery) return t('components.TrayIconBattery.noBattery');
	return t('components.TrayIconBattery.status')
		.replace('{0}', String(Math.round(batteryInfo.value.percentage)))
		.replace('{1}', String(batteryInfo.value.state));
});

/** El número de la píldora (vasak-desktop#151): «100», como en el video. */
const label = computed(() =>
	vertical.value || !showsNumbers(density.value) || !batteryInfo.value.has_battery
		? ''
		: String(Math.round(batteryInfo.value.percentage))
);

const batteryIconName = computed(() => {
	if (!batteryInfo.value.has_battery) return 'battery-missing-symbolic';

	const { percentage, is_charging: isCharging } = batteryInfo.value;

	let baseName: string;
	if (percentage < 10) baseName = 'battery-000';
	else if (percentage < 20) baseName = 'battery-010';
	else if (percentage < 30) baseName = 'battery-020';
	else if (percentage < 40) baseName = 'battery-030';
	else if (percentage < 50) baseName = 'battery-040';
	else if (percentage < 60) baseName = 'battery-050';
	else if (percentage < 70) baseName = 'battery-060';
	else if (percentage < 80) baseName = 'battery-070';
	else if (percentage < 90) baseName = 'battery-080';
	else if (percentage < 95) baseName = 'battery-090';
	else baseName = 'battery-100';

	return isCharging ? `${baseName}-charging` : baseName;
});

async function getBatteryInfoComp() {
	try {
		const info: BatteryInfo | null = await getBatteryInfo();
		if (info) {
			batteryInfo.value = info;
		} else {
			batteryInfo.value = {
				has_battery: false,
				percentage: 0,
				state: 'Unknown',
				is_present: false,
				is_charging: false,
			};
		}
	} catch (error) {
		logError('Error getting battery info:', error);
		batteryInfo.value = {
			has_battery: false,
			percentage: 0,
			state: 'Unknown',
			is_present: false,
			is_charging: false,
		};
	}
}

onMounted(async () => {
	await getBatteryInfoComp();
});

useSharedEvent<BatteryInfo>('battery-update', (payload) => {
	batteryInfo.value = payload;
});
</script>

<template>
  <!-- La píldora de la batería: el icono y el porcentaje. Sólo informa, así
       que no es un botón ni se pinta al pasar. Sin batería —un escritorio—
       no se dibuja. -->
  <PanelPill
    v-if="batteryInfo.has_battery"
    :icon="batteryIconName"
    icon-type="icon"
    :label="label"
    :interactive="false"
    :title="batteryAltText"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    :flat="hasSurface"
    class="shrink-0"
    data-battery-pill
  />
</template>
