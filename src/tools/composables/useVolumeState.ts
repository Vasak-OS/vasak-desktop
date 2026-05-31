import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, ref } from 'vue';
import type { VolumeInfo } from '@/interfaces/volume';
import { getAudioVolume, setAudioVolume, toggleAudioMute } from '@/services/core.service';
import { useEventListener } from '@/tools/event.listener';
import { logError } from '@/utils/logger';
import { calculateVolumePercentage, getVolumeIconName } from '@/utils/volume';

export function useVolumeState() {
	const volumeInfo = ref<VolumeInfo>({
		current: 0,
		min: 0,
		max: 100,
		is_muted: false,
	});
	const currentVolume = ref(0);
	const currentIcon = ref('');

	async function updateIcon(): Promise<void> {
		try {
			const percentage = calculateVolumePercentage(volumeInfo.value, currentVolume.value);
			const iconName = getVolumeIconName(volumeInfo.value.is_muted, percentage);
			currentIcon.value = await getSymbolSource(iconName);
		} catch (error) {
			logError('Error loading volume icon:', error);
		}
	}

	const volumePercentage = computed(() =>
		calculateVolumePercentage(volumeInfo.value, currentVolume.value)
	);

	async function getVolumeInfo(): Promise<void> {
		try {
			const info = await getAudioVolume();
			volumeInfo.value = info;
			currentVolume.value = info.current;
			await updateIcon();
		} catch (error) {
			logError('Error getting volume:', error);
		}
	}

	async function updateVolume(): Promise<void> {
		try {
			await setAudioVolume({ volume: currentVolume.value });
			await updateIcon();
		} catch (error) {
			logError('Error setting volume:', error);
		}
	}

	async function toggleMute(): Promise<void> {
		try {
			await toggleAudioMute();
			await getVolumeInfo();
		} catch (error) {
			logError('Error toggling mute:', error);
		}
	}

	function getPercentageClass(percentage: number) {
		if (volumeInfo.value.is_muted) return 'text-red-500';
		if (percentage > 80) return 'text-green-500';
		return '';
	}

	useEventListener<VolumeInfo>('volume-changed', async (event) => {
		volumeInfo.value = event.payload;
		currentVolume.value = event.payload.current;
		await updateIcon();
	});

	return {
		volumeInfo,
		currentVolume,
		currentIcon,
		volumePercentage,
		getVolumeInfo,
		updateVolume,
		toggleMute,
		getPercentageClass,
		updateIcon,
	};
}
