import { computed, ref } from 'vue';
import type { VolumeInfo } from '@/interfaces/volume';
import { getAudioVolume, setAudioVolume, toggleAudioMute } from '@/services/core.service';
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import { useSharedEvent } from '@/tools/event.bus';
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
	const currentIcon = useSymbol(
		computed(() => {
			const percentage = calculateVolumePercentage(volumeInfo.value, currentVolume.value);
			return getVolumeIconName(volumeInfo.value.is_muted, percentage);
		})
	);

	const volumePercentage = computed(() =>
		calculateVolumePercentage(volumeInfo.value, currentVolume.value)
	);

	async function getVolumeInfo(): Promise<void> {
		try {
			const info = await getAudioVolume();
			volumeInfo.value = info;
			currentVolume.value = info.current;
		} catch (error) {
			logError('Error getting volume:', error);
		}
	}

	// The slider emits on every input event, and each call forks a pactl
	// process. Dragging used to fire dozens of them; coalescing to the last
	// value keeps the volume responsive without the process storm.
	let volumeCommitTimer: ReturnType<typeof setTimeout> | undefined;
	const VOLUME_COMMIT_DELAY = 60;

	async function commitVolume(): Promise<void> {
		try {
			await setAudioVolume({ volume: currentVolume.value });
		} catch (error) {
			logError('Error setting volume:', error);
		}
	}

	function updateVolume(): void {
		if (volumeCommitTimer !== undefined) clearTimeout(volumeCommitTimer);
		volumeCommitTimer = setTimeout(() => {
			volumeCommitTimer = undefined;
			void commitVolume();
		}, VOLUME_COMMIT_DELAY);
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

	useSharedEvent<VolumeInfo>(
		'volume-changed',
		(payload) => {
			volumeInfo.value = payload;
			currentVolume.value = payload.current;
		},
		{ throttleMs: 16 }
	);

	return {
		volumeInfo,
		currentVolume,
		currentIcon,
		volumePercentage,
		getVolumeInfo,
		updateVolume,
		toggleMute,
		getPercentageClass,
	};
}
