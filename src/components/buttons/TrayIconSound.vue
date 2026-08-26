<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import type { VolumeInfo } from '@/interfaces/volume';
import { getAudioVolume } from '@/services/core.service';
import { toggleAudioApplet } from '@/services/window.service';
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';
import { calculateVolumePercentage, getVolumeIconName } from '@/utils/volume';

const { t } = useI18n();

const volumeInfo = ref<VolumeInfo>({
	current: 0,
	min: 0,
	max: 100,
	is_muted: false,
});
const currentVolume = ref(0);
const volumePercentage = computed(() =>
	calculateVolumePercentage(volumeInfo.value, currentVolume.value)
);
const currentIcon = useSymbol(
	computed(() => getVolumeIconName(volumeInfo.value.is_muted, volumePercentage.value))
);

async function getVolumeInfo(): Promise<void> {
	try {
		const info = (await getAudioVolume()) as VolumeInfo;
		volumeInfo.value = info;
		currentVolume.value = info.current;
	} catch (error) {
		logError('Error getting volume:', error);
	}
}

async function toggleApplet(): Promise<void> {
	try {
		await toggleAudioApplet();
	} catch (error) {
		logError('Error toggling audio applet:', error);
	}
}

onMounted(async () => {
	await getVolumeInfo();
});

useSharedEvent<VolumeInfo>(
	'volume-changed',
	(payload) => {
		volumeInfo.value = payload;
		currentVolume.value = payload.current;
	},
	{ throttleMs: 16 }
);
</script>
<template>
  <TrayIconButton
    :icon="currentIcon"
    :tooltip="volumeInfo.is_muted
      ? t('components.TrayIconSound.unmute')
      : t('components.TrayIconSound.mute')"
    :alt="volumeInfo.is_muted
      ? t('components.TrayIconSound.unmute')
      : t('components.TrayIconSound.mute')"
    :icon-class="{ 'opacity-60': volumeInfo.is_muted }"
    @click="toggleApplet"
  />
</template>