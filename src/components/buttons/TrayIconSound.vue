<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { PanelPill } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import type { VolumeInfo } from '@/interfaces/volume';
import { getAudioVolume } from '@/services/core.service';
import { toggleApplet } from '@/services/window.service';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';
import { calculateVolumePercentage, getVolumeIconName } from '@/utils/volume';

const { t } = useI18n();
const { vertical } = usePanelConfig();

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
const { isOpen } = useOpenApplet('audio');

/**
 * El número de la píldora (vasak-desktop#151): «50», como en el video. En
 * silencio no hay número: el icono tachado ya lo dice.
 */
const level = computed(() => Math.round(volumePercentage.value));
const label = computed(() =>
	vertical.value || volumeInfo.value.is_muted ? '' : String(level.value)
);
const description = computed(() =>
	volumeInfo.value.is_muted
		? t('components.TrayIconSound.muted')
		: t('components.TrayIconSound.level').replace('{0}', String(level.value))
);

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
  <!-- La píldora del volumen: el icono y el número; abre el applet de audio
       colgado de acá. -->
  <PanelPill
    ref="button"
    :icon="currentIcon"
    icon-type="icon"
    :label="label"
    :expanded="isOpen"
    :title="description"
    :accessible-label="description"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    data-volume-pill
    @click="toggleAudioApplet"
  />
</template>
