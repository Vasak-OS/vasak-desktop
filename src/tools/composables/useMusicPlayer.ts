import { invoke } from '@tauri-apps/api/core';
import { computed, ref, watch } from 'vue';
import { useIcon, useSymbol } from '@/tools/composables/useReactiveIcon';
import type { MusicInfo } from '@/interfaces/music';
import { musicNowPlaying } from '@/services/core.service';
import { useEventListener } from '@/tools/event.listener';
import { processImageUrl } from '@/utils/image';
import { logError } from '@/utils/logger';

export function useMusicPlayer() {
	const musicInfo = ref<MusicInfo>({
		title: '',
		artist: '',
		player: '',
		artUrl: '',
		status: '',
	});
	const imgSrc = ref('');
	const prevIcon = useSymbol(computed(() => 'media-seek-backward'));
	const nextIcon = useSymbol(computed(() => 'media-skip-forward'));
	const playIcon = useSymbol(computed(() => 'media-playback-start'));
	const pauseIcon = useSymbol(computed(() => 'media-playback-pause'));

	const isPlaying = computed(
		() => String(musicInfo.value?.status || '').toLowerCase() === 'playing'
	);

	function isValidBusName(name: string): boolean {
		return name.startsWith(':') ? name.length >= 4 : name.startsWith('org.mpris.MediaPlayer2.');
	}

	async function sendCommand(cmd: string): Promise<void> {
		const player = musicInfo.value?.player || '';
		if (!isValidBusName(player)) {
			return;
		}
		try {
			await invoke(cmd, { player });
		} catch (e) {
			logError(`[music] Error en comando ${cmd}:`, e);
		}
	}

	function onPrev(): void {
		sendCommand('music_previous_track');
	}
	function onNext(): void {
		sendCommand('music_next_track');
	}
	function onPlayPause(): void {
		sendCommand('music_play_pause');
	}

	const fallbackIconRef = useIcon(computed(() => 'applications-multimedia'));

	async function onImgError(): Promise<void> {
		imgSrc.value = fallbackIconRef.value || 'applications-multimedia';
	}

	async function initIcons(): Promise<void> {
		if (!imgSrc.value) {
			imgSrc.value = fallbackIconRef.value || 'applications-multimedia';
		}
	}

	async function initMusicInfo(): Promise<void> {
		try {
			musicInfo.value = await musicNowPlaying();
		} catch (error) {
			logError('[music] Error obteniendo estado inicial:', error);
		}
	}

	watch(
		() => musicInfo.value?.artUrl,
		(newUrl) => {
			const processedUrl = processImageUrl(newUrl);
			if (processedUrl) {
				imgSrc.value = processedUrl;
			} else {
				imgSrc.value = fallbackIconRef.value || 'applications-multimedia';
			}
		},
		{ immediate: true }
	);

	useEventListener<Partial<MusicInfo>>('music-playing-update', (event) => {
		const payload = event.payload || {};
		Object.assign(musicInfo.value, payload);
	});

	return {
		musicInfo,
		imgSrc,
		prevIcon,
		nextIcon,
		playIcon,
		pauseIcon,
		isPlaying,
		sendCommand,
		onPrev,
		onNext,
		onPlayPause,
		onImgError,
		initIcons,
		initMusicInfo,
	};
}
