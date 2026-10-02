/**
 * Los dobles de lo que importan `TrayMusicControl.vue` y `MusicAppletView.vue`
 * desde `@/…`. Lo puro (los tiempos, las salidas, los reproductores) es el
 * módulo de verdad; lo que habla con Tauri o con otras ventanas es un doble
 * que la prueba controla desde `music`.
 */
import { computed, defineComponent, h, ref } from 'vue';
import type { MusicInfo, PlayerRef } from '../../src/interfaces/music';

export { currentOutput, outputIcon, outputLabel } from '../../src/tools/audio-outputs';
export { activePlayerIndex, playerLabels } from '../../src/tools/music-players';
export { formatDuration, playbackStateOf } from '../../src/utils/playback';

export function blankInfo(): MusicInfo {
	return {
		player: 'org.mpris.MediaPlayer2.vlc',
		playerIdentity: 'Reproductor multimedia VLC',
		desktopEntry: 'vlc',
		status: 'Playing',
		title: 'Atardecer en la terraza del taller',
		artist: 'Los Pingüinos de Wayfire',
		album: 'Superficies translúcidas',
		artUrl: '',
		length: 245_000_000,
		position: 96_000_000,
		trackId: '1',
		canGoNext: true,
		canGoPrevious: true,
		canPlay: true,
		canPause: true,
		canSeek: true,
		canControl: true,
		canRaise: true,
		shuffle: null,
		loopStatus: null,
		volume: null,
	};
}

/** Lo que la prueba pone y lo que mira. */
export const music = {
	info: ref<MusicInfo>(blankInfo()),
	players: ref<PlayerRef[]>([]),
	devices: ref<unknown[]>([]),
	cover: ref(''),
	vertical: ref(false),
	calls: [] as Array<{ name: string; args: unknown[] }>,
	reset() {
		this.info.value = blankInfo();
		this.players.value = [];
		this.devices.value = [];
		this.cover.value = '';
		this.vertical.value = false;
		this.calls.length = 0;
	},
};

const record =
	(name: string) =>
	(...args: unknown[]) => {
		music.calls.push({ name, args });
	};

export function useMusicPlayer() {
	const position = computed(() => music.info.value.position);
	return {
		musicInfo: music.info,
		imgSrc: music.cover,
		hasCover: computed(() => Boolean(music.cover.value)),
		players: music.players,
		position,
		progress: computed(() =>
			music.info.value.length > 0 ? music.info.value.position / music.info.value.length : 0
		),
		isPlaying: computed(() => music.info.value.status.toLowerCase() === 'playing'),
		onPrev: record('onPrev'),
		onNext: record('onNext'),
		onPlayPause: record('onPlayPause'),
		onSeek: record('onSeek'),
		onImgError: record('onImgError'),
		initIcons: async () => {},
		initMusicInfo: async () => {},
		loadPlayers: async () => record('loadPlayers')(),
		selectPlayer: async (player: string) => record('selectPlayer')(player),
	};
}

export const toggleApplet = async (applet: string, button?: unknown) => {
	record('toggleApplet')(applet, button);
};

export function useOpenApplet(_applet: string) {
	return { isOpen: computed(() => false), openClasses: computed(() => ({ 'bg-ui-selected-accent': false })) };
}

export function usePanelConfig() {
	return { vertical: music.vertical };
}

export function useSharedEvent(_name: string, _handler: (payload: unknown) => void) {}

export const getAudioDevices = async () => music.devices.value;
export const setAudioDevice = async (args: unknown) => record('setAudioDevice')(args);
export const logError = () => {};

/** `AppletPopover`, sin el anclaje: la caja con la ranura. */
export default defineComponent({
	name: 'AppletPopoverDouble',
	props: { applet: { type: String, required: true } },
	emits: ['shown'],
	setup(props, { slots }) {
		return () => h('div', { 'data-applet': props.applet }, slots.default?.({ close: () => {} }));
	},
});
