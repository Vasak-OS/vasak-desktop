<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { TrayIconButton } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, type Ref, ref, watch } from 'vue';
import type { BatteryInfo } from '@/interfaces/battery';
import { fetchBatteryInfo } from '@/tools/battery.controller';
import { logError } from '@/utils/logger';

const batteryInfo: Ref<BatteryInfo> = ref({
	has_battery: false,
	percentage: 0,
	state: 'Unknown',
	is_present: false,
	is_charging: false,
});

const batteryIconSrc: Ref<string> = ref('');
const unlistenBattery: Ref<(() => void) | null> = ref(null);

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
	'text-vsk-primary': batteryInfo.value.percentage >= 50 && !batteryInfo.value.is_charging,
}));

async function updateIcon() {
	const getIconName = () => {
		if (!batteryInfo.value.has_battery) {
			return 'battery-missing-symbolic';
		}

		const percentage = batteryInfo.value.percentage;
		const isCharging = batteryInfo.value.is_charging;

		// Iconos de carga
		if (isCharging) {
			if (percentage >= 90) return 'battery-full-charging-symbolic';
			if (percentage >= 70) return 'battery-good-charging-symbolic';
			if (percentage >= 40) return 'battery-low-charging-symbolic';
			if (percentage >= 20) return 'battery-caution-charging-symbolic';
			return 'battery-empty-charging-symbolic';
		}

		// Iconos normales
		if (percentage >= 90) return 'battery-full-symbolic';
		if (percentage >= 70) return 'battery-good-symbolic';
		if (percentage >= 40) return 'battery-low-symbolic';
		if (percentage >= 20) return 'battery-caution-symbolic';
		return 'battery-empty-symbolic';
	};

	try {
		batteryIconSrc.value = await getSymbolSource(getIconName());
	} catch (error) {
		logError('Error loading battery icon:', error);
		try {
			batteryIconSrc.value = await getSymbolSource('battery-symbolic');
		} catch (fallbackError) {
			logError('Error loading fallback battery icon:', fallbackError);
		}
	}
}

watch(
	[
		() => batteryInfo.value.has_battery,
		() => batteryInfo.value.percentage,
		() => batteryInfo.value.is_charging,
		() => batteryInfo.value.state,
	],
	updateIcon,
	{ immediate: true }
);

async function getBatteryInfo() {
	try {
		const info: BatteryInfo | null = await fetchBatteryInfo();
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
		await updateIcon();
	} catch (error) {
		logError('Error getting battery info:', error);
		batteryInfo.value.has_battery = false;
		await updateIcon();
	}
}

async function toggleBatteryInfo() {
	// Toggle behavior for battery info display
}

onMounted(async () => {
	unlistenBattery.value = await listen('battery-update', (event) => {
		batteryInfo.value = event.payload as BatteryInfo;
		updateIcon();
	});
	await getBatteryInfo();
});

onUnmounted(() => {
	if (unlistenBattery.value) {
		unlistenBattery.value();
	}
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

