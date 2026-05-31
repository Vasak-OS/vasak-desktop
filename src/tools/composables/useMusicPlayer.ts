import { invoke } from '@tauri-apps/api/core';
import { getIconSource, getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, ref, watch } from 'vue';
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
	const prevIcon = ref('');
	const nextIcon = ref('');
	const playIcon = ref('');
	const pauseIcon = ref('');

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

	async function onImgError(): Promise<void> {
		imgSrc.value = await getIconSource('applications-multimedia');
	}

	async function initIcons(): Promise<void> {
		imgSrc.value = await getIconSource('applications-multimedia');
		prevIcon.value = await getSymbolSource('media-seek-backward');
		nextIcon.value = await getSymbolSource('media-skip-forward');
		playIcon.value = await getSymbolSource('media-playback-start');
		pauseIcon.value = await getSymbolSource('media-playback-pause');
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
		async (newUrl) => {
			const processedUrl = processImageUrl(newUrl);
			if (processedUrl) {
				imgSrc.value = processedUrl;
			} else {
				imgSrc.value = await getIconSource('applications-multimedia');
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
