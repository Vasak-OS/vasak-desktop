<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import SliderControl from '@/components/base/SliderControl.vue';

interface BrightnessInfo {
  current: number;
  min: number;
  max: number;
}

const brightnessInfo: Ref<BrightnessInfo> = ref<BrightnessInfo>({
	current: 100,
	min: 0,
	max: 100,
});

const currentBrightness: Ref<number> = ref(100);
const currentIcon: Ref<string> = ref('');
let unlisten: (() => void) | null = null;
let setDebitTimeout: ReturnType<typeof setTimeout> | null = null;

async function updateIcon() {
	try {
		let iconName: string;

		if (brightnessPercentage.value > 66) {
			iconName = 'display-brightness-high-symbolic';
		} else if (brightnessPercentage.value > 33) {
			iconName = 'display-brightness-medium-symbolic';
		} else {
			iconName = 'display-brightness-low-symbolic';
		}

		currentIcon.value = await getSymbolSource(iconName);
	} catch (error) {
		console.error('Error loading brightness icon:', error);
	}
}

const brightnessPercentage = computed(() => {
	if (brightnessInfo.value.max <= 0) return 0;
	const range = brightnessInfo.value.max - brightnessInfo.value.min;
	const loading = currentBrightness.value - brightnessInfo.value.min;
	return Math.round((loading / range) * 100);
});

async function getBrightnessInfo() {
	try {
		const info = await invoke<BrightnessInfo>('get_backlight_brightness');
		brightnessInfo.value = info;
		currentBrightness.value = info.current;
		await updateIcon();
	} catch (error) {
		console.error('Error getting brightness:', error);
	}
}

async function updateBrightness() {
	try {
		if (setDebitTimeout) {
			clearTimeout(setDebitTimeout);
		}

		setDebitTimeout = setTimeout(async () => {
			await invoke('set_backlight_brightness', {
				brightness: Number(currentBrightness.value),
			});
			await updateIcon();
		}, 50);
	} catch (error) {
		console.error('Error setting brightness:', error);
	}
}

const getPercentageClass = (percentage: number) => {
	if (percentage > 80) return 'text-yellow-500';
	if (percentage < 20) return 'text-orange-500';
	return '';
};

onMounted(async () => {
	unlisten = await listen('brightness-changed', async (event) => {
		brightnessInfo.value = event.payload as BrightnessInfo;
		currentBrightness.value = (event.payload as BrightnessInfo).current;
		await updateIcon();
	});
	await getBrightnessInfo();
});

onUnmounted(() => {
	if (unlisten) {
		unlisten();
	}
	if (setDebitTimeout) {
		clearTimeout(setDebitTimeout);
	}
});
</script>

<template>
  <SliderControl
    :icon="currentIcon"
    :alt="'Brillo: ' + brightnessPercentage + '%'"
    :tooltip="'Brillo: ' + brightnessPercentage + '%'"
    v-model="currentBrightness"
    :min="brightnessInfo.min"
    :max="brightnessInfo.max"
    :show-button="false"
    :get-percentage-class="getPercentageClass"
    @update:model-value="updateBrightness"
  />
</template>
