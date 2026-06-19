<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useSymbol } from '@/tools/composables/useReactiveIcon';
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
let setDebitTimeout: ReturnType<typeof setTimeout> | null = null;

const brightnessPercentage = computed(() => {
	if (brightnessInfo.value.max <= 0) return 0;
	const range = brightnessInfo.value.max - brightnessInfo.value.min;
	const loading = currentBrightness.value - brightnessInfo.value.min;
	return Math.round((loading / range) * 100);
});

const currentIcon = useSymbol(computed(() => {
	if (brightnessPercentage.value > 66) return 'display-brightness-high-symbolic';
	if (brightnessPercentage.value > 33) return 'display-brightness-medium-symbolic';
	return 'display-brightness-low-symbolic';
}));

async function getBrightnessInfo() {
	try {
		const info = await fetchBrightnessInfo();
		brightnessInfo.value = info;
		currentBrightness.value = info.current;
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

useEventListener('brightness-changed', async (event: { payload: Record<string, number> }) => {
	const p = event.payload;
	if (p.current !== undefined) {
		brightnessInfo.value = p as BrightnessInfo;
		currentBrightness.value = p.current;
	} else if (p.value !== undefined && p.max !== undefined && p.max > 0) {
		const current = p.percentage ?? Math.round((p.value / p.max) * 100);
		brightnessInfo.value = { current, max: 100, min: 0 };
		currentBrightness.value = current;
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
