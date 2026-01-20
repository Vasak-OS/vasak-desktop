<template>
  <div
    class="p-1 rounded-vsk relative hover:bg-vsk-primary/30 group"
    @click="toggleBatteryInfo"
    @mouseenter="showTooltip = true"
    @mouseleave="showTooltip = false"
  >
    <img
      :src="currentIcon"
      :alt="batteryAltText"
      :title="batteryTitle"
      class="m-auto h-[22px] w-auto transition-all duration-300"
      :class="{ 
        'opacity-60': !batteryInfo.has_battery,
        'animate-pulse': batteryInfo.is_charging,
        'text-red-500': batteryInfo.percentage < 20 && !batteryInfo.is_charging
      }"
    />
    
    <div 
      v-if="batteryInfo.has_battery"
      class="absolute top-1 left-1/2 transform -translate-x-1/2 text-xs font-semibold p-1 rounded-vsk transition-all duration-300 pointer-events-none background"
      :class="{
        'text-green-400': batteryInfo.is_charging,
        'text-red-400': batteryInfo.percentage < 20 && !batteryInfo.is_charging,
        'text-yellow-400': batteryInfo.percentage < 50 && batteryInfo.percentage >= 20 && !batteryInfo.is_charging,
        'text-vsk-primary': batteryInfo.percentage >= 50 && !batteryInfo.is_charging,
        'opacity-0 -translate-y-2': !showTooltip,
        'opacity-100 translate-y-0': showTooltip
      }"
    >
      {{ Math.round(batteryInfo.percentage) }}%
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, Ref, onUnmounted } from 'vue';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { listen } from '@tauri-apps/api/event';
import type { BatteryInfo } from '@/interfaces/battery';
import { fetchBatteryInfo } from '@/tools/battery.controller';

const batteryInfo: Ref<BatteryInfo> = ref({
	has_battery: false,
	percentage: 0,
	state: 'Unknown',
	is_present: false,
	is_charging: false,
});
const currentIcon: Ref<string> = ref('');
const showPercentage: Ref<boolean> = ref(true);
const showTooltip: Ref<boolean> = ref(false);
const unlistenBattery: Ref<(() => void) | null> = ref(null);

const batteryAltText = computed(() => {
	if (!batteryInfo.value.has_battery) return 'No battery detected';
	return `Battery ${Math.round(batteryInfo.value.percentage)}% - ${batteryInfo.value.state}`;
});

const batteryTitle = computed(() => {
	if (!batteryInfo.value.has_battery) return 'No battery detected';
  
	let title = `Battery: ${Math.round(batteryInfo.value.percentage)}% (${batteryInfo.value.state})`;
  
	if (batteryInfo.value.time_to_empty && !batteryInfo.value.is_charging) {
		const hours = Math.floor(batteryInfo.value.time_to_empty / 3600);
		const minutes = Math.floor((batteryInfo.value.time_to_empty % 3600) / 60);
		title += `\nTime remaining: ${hours}h ${minutes}m`;
	}
  
	if (batteryInfo.value.time_to_full && batteryInfo.value.is_charging) {
		const hours = Math.floor(batteryInfo.value.time_to_full / 3600);
		const minutes = Math.floor((batteryInfo.value.time_to_full % 3600) / 60);
		title += `\nTime to full: ${hours}h ${minutes}m`;
	}
  
	if (batteryInfo.value.vendor && batteryInfo.value.model) {
		title += `\n${batteryInfo.value.vendor} ${batteryInfo.value.model}`;
	}
  
	return title;
});

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
		currentIcon.value = await getSymbolSource(getIconName());
	} catch (error) {
		console.error('Error loading battery icon:', error);
		try {
			currentIcon.value = await getSymbolSource('battery-symbolic');
		} catch (fallbackError) {
			console.error('Error loading fallback battery icon:', fallbackError);
		}
	}
}

watch([
	() => batteryInfo.value.has_battery,
	() => batteryInfo.value.percentage,
	() => batteryInfo.value.is_charging,
	() => batteryInfo.value.state
], updateIcon, { immediate: true });

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
		console.error('Error getting battery info:', error);
		batteryInfo.value.has_battery = false;
		await updateIcon();
	}
}

async function toggleBatteryInfo() {
	// Aquí podrías mostrar un panel detallado de batería o alternar el porcentaje
	showPercentage.value = !showPercentage.value;
}

onMounted(async () => {
	unlistenBattery.value = await listen('battery-update', (event) => {
		console.log('Battery update event received:', event.payload);
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
