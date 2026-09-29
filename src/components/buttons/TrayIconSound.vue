<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { TrayIconButton } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import type { VolumeInfo } from '@/interfaces/volume';
import { getAudioVolume } from '@/services/core.service';
import { toggleApplet } from '@/services/window.service';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
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
const currentIcon = computed(() =>
	getVolumeIconName(volumeInfo.value.is_muted, volumePercentage.value)
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

const button = ref<unknown>(null);
const { openClasses } = useOpenApplet('audio');

async function toggleAudioApplet(): Promise<void> {
	try {
		await toggleApplet('audio', button.value);
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
    ref="button"
    :name="currentIcon"
    :tooltip="volumeInfo.is_muted
      ? t('components.TrayIconSound.unmute')
      : t('components.TrayIconSound.mute')"
    :alt="volumeInfo.is_muted
      ? t('components.TrayIconSound.unmute')
      : t('components.TrayIconSound.mute')"
    :icon-class="{ 'opacity-60': volumeInfo.is_muted }"
    :custom-class="openClasses"
    @click="toggleAudioApplet"
  />
</template>