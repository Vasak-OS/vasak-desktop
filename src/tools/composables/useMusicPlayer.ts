import { invoke } from '@tauri-apps/api/core';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, onUnmounted, ref, watch } from 'vue';
import type { MusicInfo, PlayerRef } from '@/interfaces/music';
import { musicNowPlaying } from '@/services/core.service';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';
import { nextLoop, positionFromRatio, positionNow, progressRatio } from '@/utils/playback';

/** El estado de un reproductor que no existe todavía. */
function emptyInfo(): MusicInfo {
	return {
		player: '',
		playerIdentity: '',
		status: '',
		title: '',
		artist: '',
		album: '',
		artUrl: '',
		length: 0,
		position: 0,
		trackId: '',
		canGoNext: false,
		canGoPrevious: false,
		canPlay: false,
		canPause: false,
		canSeek: false,
		canControl: false,
		canRaise: false,
		shuffle: null,
		loopStatus: null,
		volume: null,
	};
}

export function useMusicPlayer() {
	const musicInfo = ref<MusicInfo>(emptyInfo());
	const imgSrc = ref('');
	const players = ref<PlayerRef[]>([]);
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
	const stopIcon = 'media-playback-stop';
	const shuffleIcon = 'media-playlist-shuffle';
	const loopIcon = 'media-playlist-repeat';
	const loopOneIcon = 'media-playlist-repeat-song';
	const volumeIcon = 'audio-volume-high';
	const raiseIcon = 'window-new';

	const isPlaying = computed(
		() => String(musicInfo.value?.status || '').toLowerCase() === 'playing'
	);

	function isValidBusName(name: string): boolean {
		return name.startsWith(':') ? name.length >= 4 : name.startsWith('org.mpris.MediaPlayer2.');
	}

	async function sendCommand(cmd: string, extra: Record<string, unknown> = {}): Promise<void> {
		const player = musicInfo.value?.player || '';
		if (!isValidBusName(player)) {
			return;
		}
		try {
			await invoke(cmd, { player, ...extra });
		} catch (e) {
			logError(`[music] Error en comando ${cmd}:`, e);
		}
	}

	function onPrev(): void {
		if (!musicInfo.value.canGoPrevious) return;
		sendCommand('music_previous_track');
	}
	function onNext(): void {
		if (!musicInfo.value.canGoNext) return;
		sendCommand('music_next_track');
	}
	function onPlayPause(): void {
		sendCommand('music_play_pause');
	}
	function onStop(): void {
		sendCommand('music_stop');
	}
	function onRaise(): void {
		if (!musicInfo.value.canRaise) return;
		sendCommand('music_raise');
	}

	/**
	 * Salta a un punto de la pista, dicho como fracción de la barra.
	 *
	 * La posición se adelanta acá mismo sin esperar la respuesta: el reproductor
	 * tarda en confirmar y, hasta que lo hace, el sondeo sigue mandando la
	 * posición vieja. Sin esto la barra vuelve atrás sola después de cada salto.
	 */
	function onSeek(ratio: number): void {
		if (!musicInfo.value.canSeek || musicInfo.value.length <= 0) return;
		const target = positionFromRatio(ratio, musicInfo.value.length);
		musicInfo.value.position = target;
		positionBase.value = target;
		positionAt.value = Date.now();
		sendCommand('music_set_position', { position: target });
	}

	function onVolume(volume: number): void {
		if (musicInfo.value.volume === null) return;
		musicInfo.value.volume = volume;
		sendCommand('music_set_volume', { volume });
	}

	function onShuffle(): void {
		if (musicInfo.value.shuffle === null) return;
		const wanted = !musicInfo.value.shuffle;
		musicInfo.value.shuffle = wanted;
		sendCommand('music_set_shuffle', { shuffle: wanted });
	}

	function onLoop(): void {
		if (musicInfo.value.loopStatus === null) return;
		const wanted = nextLoop(musicInfo.value.loopStatus);
		musicInfo.value.loopStatus = wanted;
		sendCommand('music_set_loop', { status: wanted });
	}

	/** La lista de reproductores, para el selector. */
	async function loadPlayers(): Promise<void> {
		try {
			players.value = await invoke<PlayerRef[]>('music_players');
		} catch (e) {
			logError('[music] Error listando reproductores:', e);
		}
	}

	/** Elige uno a mano; con el vacío vuelve la elección automática. */
	async function selectPlayer(player: string): Promise<void> {
		try {
			await invoke('music_select_player', { player });
			await loadPlayers();
		} catch (e) {
			logError('[music] Error eligiendo reproductor:', e);
		}
	}

	/**
	 * La tapa de respaldo, cuando no hay carátula o la que hay no carga.
	 *
	 * Acá sí hace falta una **ruta** y no un nombre: `imgSrc` es la carátula que
	 * manda el reproductor y se dibuja con un `img` común, así que el respaldo
	 * tiene que ser algo que ese mismo `img` pueda mostrar. Es el único lugar
	 * del escritorio donde sigue haciendo falta resolver a mano.
	 */
	const fallbackCover = ref('');
	/** Si lo que se está mostrando es la carátula de verdad o el respaldo. */
	const hasCover = ref(false);

	async function resolveFallbackCover(): Promise<void> {
		if (!fallbackCover.value) {
			fallbackCover.value = (await getIconSource('applications-multimedia')) || '';
		}
	}

	/**
	 * El `blob:` de la carátula, que hay que soltar a mano.
	 *
	 * Vive mientras viva el documento aunque nadie lo mire, y son varios cientos
	 * de kB por pista. Se suelta al cambiar de carátula y al desmontar.
	 */
	let coverObjectUrl = '';
	/**
	 * Cuál es el pedido de carátula que vale.
	 *
	 * Dos pistas seguidas son dos pedidos en vuelo, y el primero puede contestar
	 * último: sin esto, la carátula vieja pisa a la nueva —o la suelta— y queda
	 * puesta la que no es. Después de desmontar tampoco se adopta ninguna, que
	 * si no se crea un `blob:` que ya nadie va a soltar.
	 */
	let coverRequest = 0;
	let disposed = false;

	function releaseCover(): void {
		if (coverObjectUrl) {
			URL.revokeObjectURL(coverObjectUrl);
			coverObjectUrl = '';
		}
	}

	async function showFallbackCover(): Promise<void> {
		await resolveFallbackCover();
		releaseCover();
		hasCover.value = false;
		imgSrc.value = fallbackCover.value;
	}

	async function onImgError(): Promise<void> {
		await showFallbackCover();
	}

	/**
	 * La carátula, por el camino que corresponda a lo que mandó el reproductor.
	 *
	 * Un archivo del disco **no** se le da al WebView como ruta: el protocolo de
	 * assets sólo sirve lo que esté dentro del alcance declarado y contesta 403 a
	 * lo demás, que es por qué la carátula de Chromium —que la deja en `/tmp`— no
	 * se veía nunca. Los bytes vienen por el IPC y se arma un `blob:` de este
	 * mismo origen. Lo remoto sí lo carga el WebView, que ya trae TLS.
	 */
	async function resolveCover(url: string): Promise<void> {
		const request = ++coverRequest;
		const clean = (url || '').trim();
		if (!clean) {
			await showFallbackCover();
			return;
		}

		if (/^(https?|data):/.test(clean)) {
			releaseCover();
			hasCover.value = true;
			imgSrc.value = clean;
			return;
		}

		try {
			const bytes = await invoke<ArrayBuffer>('music_artwork', { url: clean });
			// Mientras se leía puede haber cambiado la pista, o haberse cerrado la
			// ventana: esta carátula ya no es la que va.
			if (disposed || request !== coverRequest) return;
			const fresh = URL.createObjectURL(new Blob([bytes]));
			releaseCover();
			coverObjectUrl = fresh;
			hasCover.value = true;
			imgSrc.value = fresh;
		} catch (e) {
			if (disposed || request !== coverRequest) return;
			logError('[music] La carátula no se pudo leer:', e);
			await showFallbackCover();
		}
	}

	async function initIcons(): Promise<void> {
		await resolveFallbackCover();
		if (!imgSrc.value) {
			imgSrc.value = fallbackCover.value;
		}
	}

	async function initMusicInfo(): Promise<void> {
		try {
			Object.assign(musicInfo.value, await musicNowPlaying());
		} catch (error) {
			logError('[music] Error obteniendo estado inicial:', error);
		}
	}

	watch(
		[() => musicInfo.value?.artUrl, fallbackCover],
		([url]) => {
			void resolveCover(String(url || ''));
		},
		{ immediate: true }
	);

	/**
	 * La posición, que el reproductor sólo manda cuando le preguntan.
	 *
	 * `Position` es la única propiedad que MPRIS exime de avisar sus cambios, así
	 * que entre dos sondeos —dos segundos— nadie diría que la música avanza. Lo
	 * que se guarda es la última que llegó y cuándo llegó; el resto es reloj.
	 */
	const positionBase = ref(0);
	const positionAt = ref(Date.now());
	const now = ref(Date.now());

	const clock = setInterval(() => {
		now.value = Date.now();
	}, 500);
	onUnmounted(() => {
		disposed = true;
		clearInterval(clock);
		releaseCover();
	});

	watch(
		() => musicInfo.value?.position,
		(micros) => {
			positionBase.value = Number(micros) || 0;
			positionAt.value = Date.now();
			now.value = Date.now();
		},
		{ immediate: true }
	);

	const position = computed(() =>
		positionNow(
			positionBase.value,
			now.value - positionAt.value,
			isPlaying.value,
			musicInfo.value.length
		)
	);
	const progress = computed(() => progressRatio(position.value, musicInfo.value.length));

	useSharedEvent<Partial<MusicInfo>>('music-playing-update', (payload) => {
		Object.assign(musicInfo.value, payload || {});
	});

	return {
		musicInfo,
		imgSrc,
		hasCover,
		players,
		prevIcon,
		nextIcon,
		playIcon,
		pauseIcon,
		stopIcon,
		shuffleIcon,
		loopIcon,
		loopOneIcon,
		volumeIcon,
		raiseIcon,
		isPlaying,
		position,
		progress,
		sendCommand,
		onPrev,
		onNext,
		onPlayPause,
		onStop,
		onRaise,
		onSeek,
		onVolume,
		onShuffle,
		onLoop,
		onImgError,
		loadPlayers,
		selectPlayer,
		initIcons,
		initMusicInfo,
	};
}
