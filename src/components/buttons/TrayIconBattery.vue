<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import { computed, onMounted, ref } from 'vue';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import type { BatteryInfo } from '@/interfaces/battery';
import { getBatteryInfo } from '@/services/core.service';
import { useEventListener } from '@/tools/event.listener';
import { logError } from '@/utils/logger';

const batteryInfo = ref<BatteryInfo>({
	has_battery: false,
	percentage: 0,
	state: 'Unknown',
	is_present: false,
	is_charging: false,
});

const batteryAltText = computed(() => {
	if (!batteryInfo.value.has_battery) return 'No battery detected';
	return `Battery ${Math.round(batteryInfo.value.percentage)}% - ${batteryInfo.value.state}`;
});

const tooltipClass = computed(() => ({
	'text-green-400': batteryInfo.value.is_charging,
	'text-red-400': batteryInfo.value.percentage < 20 && !batteryInfo.value.is_charging,
	'text-yellow-400':
		batteryInfo.value.percentage < 50 &&
		batteryInfo.value.percentage >= 20 &&
		!batteryInfo.value.is_charging,
	'text-primary': batteryInfo.value.percentage >= 50 && !batteryInfo.value.is_charging,
}));

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

const batteryIconSrc = useSymbol(batteryIconName);

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

async function toggleBatteryInfo() {
	// Toggle behavior for battery info display
}

onMounted(async () => {
	await getBatteryInfoComp();
});

useEventListener('battery-update', (event) => {
	batteryInfo.value = event.payload as BatteryInfo;
});
</script>

<template>
  <TrayIconButton
    :icon="batteryIconSrc"
    :alt="batteryAltText"
    :tooltip="batteryAltText"
    :show-custom-tooltip="batteryInfo.has_battery"
    :custom-tooltip-text="batteryInfo.has_battery ? Math.round(batteryInfo.percentage) + '%' : ''"
    :tooltip-class="tooltipClass"
    :icon-class="{
      'opacity-60': !batteryInfo.has_battery,
      'animate-pulse': batteryInfo.is_charging,
    }"
    @click="toggleBatteryInfo"
  />
</template>

