import { invoke } from '@tauri-apps/api/core';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, ref, watch } from 'vue';
import type { MusicInfo } from '@/interfaces/music';
import { musicNowPlaying } from '@/services/core.service';
import { useSharedEvent } from '@/tools/event.bus';
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
	/**
	 * Los **nombres** de los iconos del transporte, no sus rutas.
	 *
	 * Un composable no puede devolver un componente: de acá salen los nombres y
	 * los dibuja `ThemeIcon` en cada vista que los use.
	 */
	const prevIcon = 'media-seek-backward';
	const nextIcon = 'media-skip-forward';
	const playIcon = 'media-playback-start';
	const pauseIcon = 'media-playback-pause';

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

	/**
	 * La tapa de respaldo, cuando no hay carátula o la que hay no carga.
	 *
	 * Acá sí hace falta una **ruta** y no un nombre: `imgSrc` es la carátula que
	 * manda el reproductor y se dibuja con un `img` común, así que el respaldo
	 * tiene que ser algo que ese mismo `img` pueda mostrar. Es el único lugar
	 * del escritorio donde sigue haciendo falta resolver a mano.
	 */
	const tapaDeRespaldo = ref('');

	async function resolverLaTapaDeRespaldo(): Promise<void> {
		if (!tapaDeRespaldo.value) {
			tapaDeRespaldo.value = (await getIconSource('applications-multimedia')) || '';
		}
	}

	async function onImgError(): Promise<void> {
		await resolverLaTapaDeRespaldo();
		imgSrc.value = tapaDeRespaldo.value;
	}

	async function initIcons(): Promise<void> {
		await resolverLaTapaDeRespaldo();
		if (!imgSrc.value) {
			imgSrc.value = tapaDeRespaldo.value;
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
		[() => musicInfo.value?.artUrl, tapaDeRespaldo],
		([newUrl, respaldo]) => {
			const processedUrl = processImageUrl(newUrl);
			imgSrc.value = processedUrl || respaldo;
		},
		{ immediate: true }
	);

	useSharedEvent<Partial<MusicInfo>>('music-playing-update', (payload) => {
		Object.assign(musicInfo.value, payload || {});
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
