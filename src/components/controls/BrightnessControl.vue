<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import {
	getBrightnessInfo as fetchBrightnessInfo,
	setBrightnessInfo,
} from '@/services/core.service';
import { useEventListener } from '@/tools/event.listener';
import { logError } from '@/utils/logger';
import SliderControl from '../forms/SliderControl.vue';

interface BrightnessInfo {
	current: number;
	min: number;
	max: number;
}

const brightnessInfo = ref<BrightnessInfo>({
	current: 100,
	min: 0,
	max: 100,
});

const currentBrightness = ref(100);
const currentIcon = ref('');
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
		logError('Error loading brightness icon:', error);
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
		const info = await fetchBrightnessInfo();
		brightnessInfo.value = info;
		currentBrightness.value = info.current;
		await updateIcon();
	} catch (error) {
		logError('Error getting brightness:', error);
	}
}

async function updateBrightness() {
	try {
		if (setDebitTimeout) {
			clearTimeout(setDebitTimeout);
		}

		setDebitTimeout = setTimeout(async () => {
			await setBrightnessInfo({
				brightness: Number(currentBrightness.value),
			});
			await updateIcon();
		}, 50);
	} catch (error) {
		logError('Error setting brightness:', error);
	}
}

const getPercentageClass = (percentage: number) => {
	if (percentage > 80) return 'text-yellow-500';
	if (percentage < 20) return 'text-orange-500';
	return '';
};

onMounted(async () => {
	await getBrightnessInfo();
});

onUnmounted(() => {
	if (setDebitTimeout) {
		clearTimeout(setDebitTimeout);
	}
});

useEventListener<BrightnessInfo>('brightness-changed', async (event) => {
	brightnessInfo.value = event.payload;
	currentBrightness.value = event.payload.current;
	await updateIcon();
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
