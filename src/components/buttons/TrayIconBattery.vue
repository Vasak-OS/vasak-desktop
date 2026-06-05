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

	if (isCharging) {
		if (percentage >= 90) return 'battery-full-charging-symbolic';
		if (percentage >= 70) return 'battery-good-charging-symbolic';
		if (percentage >= 40) return 'battery-low-charging-symbolic';
		if (percentage >= 20) return 'battery-caution-charging-symbolic';
		return 'battery-empty-charging-symbolic';
	}

	if (percentage >= 90) return 'battery-full-symbolic';
	if (percentage >= 70) return 'battery-good-symbolic';
	if (percentage >= 40) return 'battery-low-symbolic';
	if (percentage >= 20) return 'battery-caution-symbolic';
	return 'battery-empty-symbolic';
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

