<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	Badge,
	NowPlayingCard,
	OptionGroup,
	type OptionGroupOption,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
import type { AudioDevice } from '@/interfaces/audio-device';
import { getAudioDevices, setAudioDevice } from '@/services/core.service';
import { currentOutput, outputIcon, outputLabel } from '@/tools/audio-outputs';
import { useMusicPlayer } from '@/tools/composables/useMusicPlayer';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';
import { formatDuration, playbackStateOf } from '@/utils/playback';

/**
 * El reproductor que se despliega desde el control de música del panel.
 *
 * Qué suena, por dónde va y por qué salida sale, sin abrir el reproductor de
 * turno. Todo lo de la música sale de `useMusicPlayer`, el mismo que usan el
 * control del panel y el widget: no hay un segundo camino a MPRIS. La salida de
 * audio es la de `get_audio_devices` / `set_audio_device`, la misma del applet
 * de audio.
 *
 * Muestra el reproductor que ya elige `music.rs`, el activo. Con varios
 * abiertos, cambiar entre ellos queda para cuando se decida cómo (ver el
 * issue #131).
 *
 * El espacio del ecualizador (vasak-wireplumber-modules#10) es la ranura
 * `footer` de la tarjeta, que sin contenido no se dibuja: hasta que exista, la
 * tarjeta termina en el transporte.
 */

const { t } = useI18n();

const {
	musicInfo,
	imgSrc,
	hasCover,
	position,
	isPlaying,
	onPrev,
	onNext,
	onPlayPause,
	onSeek,
	onImgError,
	initIcons,
	initMusicInfo,
} = useMusicPlayer();

const state = computed(() => playbackStateOf(musicInfo.value.status));

/**
 * La carátula, o nada: sin carátula el disco dibuja el icono de la aplicación
 * sobre la superficie. La tapa de respaldo de `useMusicPlayer` es una imagen
 * para un `img` común, y acá el disco ya sabe dibujar su propio respaldo.
 */
const cover = computed(() => (hasCover.value ? imgSrc.value : null));

/** El icono del reproductor, que es el nombre de su `.desktop`. */
const appIcon = computed(() => musicInfo.value.desktopEntry || 'applications-multimedia');

/** Reproducir o pausar, según lo que el reproductor diga que acepta ahora. */
const canPlayPause = computed(() => {
	const info = musicInfo.value;
	if (!info.player) return false;
	return isPlaying.value ? info.canPause : info.canPlay;
});

/** La barra emite en microsegundos; el composable salta por fracción. */
function seekTo(micros: number): void {
	const length = musicInfo.value.length;
	if (length > 0) onSeek(micros / length);
}

const via = computed(() =>
	musicInfo.value.playerIdentity
		? t('views.musicApplet.via').replace('{0}', () => musicInfo.value.playerIdentity)
		: ''
);

// ── La salida de audio ──────────────────────────────────────────────────────

const devices = ref<AudioDevice[]>([]);
const output = computed(() => currentOutput(devices.value));
/** Lo que oye un lector de pantalla en el chip: qué es, y cuál está puesta. */
const outputAnnouncement = computed(() => {
	const name = output.value ? outputLabel(output.value) : '';
	return t('views.musicApplet.currentOutput').replace('{0}', () => name);
});
const pickingOutput = ref(false);
/** La salida pedida y todavía sin confirmar, para no aceptar dos clics. */
const switching = ref<string | null>(null);

async function loadDevices(): Promise<void> {
	try {
		devices.value = await getAudioDevices<AudioDevice[]>();
	} catch (error) {
		logError('[MusicApplet] no se pudieron leer las salidas de audio:', error);
	}
}

useSharedEvent<AudioDevice[]>('audio-devices-changed', (payload) => {
	devices.value = payload ?? [];
});

async function chooseOutput(device: AudioDevice): Promise<void> {
	if (switching.value) return;
	if (device.is_default) {
		pickingOutput.value = false;
		return;
	}
	switching.value = device.id;
	try {
		await setAudioDevice({ deviceId: device.id });
		await loadDevices();
		pickingOutput.value = false;
	} catch (error) {
		logError('[MusicApplet] no se pudo cambiar la salida:', error);
	} finally {
		switching.value = null;
	}
}

/**
 * Cada vez que vuelve a la vista.
 *
 * Esconder el applet no destruye el webview, así que Vue no se monta de nuevo:
 * lo que cambió mientras estaba escondido —otra salida, otra pista— se vuelve a
 * pedir acá. Y el selector de salida se cierra, para abrir siempre en la
 * tarjeta.
 */
async function refresh(): Promise<void> {
	pickingOutput.value = false;
	await Promise.all([initMusicInfo(), loadDevices()]);
}

onMounted(async () => {
	await initIcons();
	await refresh();
});

/** Las salidas como opciones del grupo, con el icono de cada una. */
const outputOptions = computed<OptionGroupOption<string>[]>(() =>
	devices.value.map((device) => ({
		value: device.id,
		label: outputLabel(device),
		icon: outputIcon(device),
		iconType: 'symbol',
	}))
);

function onOutputChange(id: string): void {
	const device = devices.value.find((candidate) => candidate.id === id);
	if (device) void chooseOutput(device);
}
</script>

<template>
	<AppletPopover applet="music" @shown="refresh">
		<div class="relative h-full min-h-0">
			<NowPlayingCard
				:title="musicInfo.title"
				:artist="musicInfo.artist"
				:album="musicInfo.album"
				:cover-src="cover"
				:fallback-icon="appIcon"
				:state="state"
				:position="position"
				:duration="musicInfo.length"
				:format="formatDuration"
				:seek-step="5_000_000"
				:can-seek="musicInfo.canSeek"
				:can-go-previous="musicInfo.canGoPrevious"
				:can-go-next="musicInfo.canGoNext"
				:can-play-pause="canPlayPause"
				:previous-label="t('views.musicApplet.previous')"
				:next-label="t('views.musicApplet.next')"
				:play-label="t('views.musicApplet.play')"
				:pause-label="t('views.musicApplet.pause')"
				:seek-label="t('views.musicApplet.seek')"
				:by-artist-label="t('views.musicApplet.byArtist')"
				:nothing-playing-label="t('views.musicApplet.nothingPlaying')"
				@previous="onPrev"
				@next="onNext"
				@toggle="onPlayPause"
				@seek="seekTo"
				@cover-error="onImgError"
			>
				<template #details>
					<!-- La salida y por dónde suena: un botón chico de la librería que
					     abre el selector, y una insignia que sólo informa. -->
					<ActionButton
						v-if="output"
						:label="outputLabel(output)"
						:icon="outputIcon(output)"
						variant="secondary"
						size="sm"
						:title="t('views.musicApplet.chooseOutput')"
						:aria-label="outputAnnouncement"
						:aria-expanded="pickingOutput"
						@click="pickingOutput = true"
					/>
					<Badge v-if="via" :label="via" :title="via" />
				</template>
			</NowPlayingCard>

			<!-- El selector de salida, encima de la tarjeta: el applet mide lo
			     que mide y una lista que empujara la tarjeta la sacaría de la
			     ventana. Es `OptionGroup` de la librería, el mismo del applet de
			     audio: un `radiogroup` con las flechas y un solo Tab. -->
			<div
				v-if="pickingOutput"
				class="absolute inset-0 z-10 flex min-h-0 flex-col gap-2 rounded-corner-m bg-ui-surface p-2"
			>
				<div class="flex min-w-0 items-center gap-2">
					<ActionButton
						label=""
						icon="go-previous"
						:icon-alt="t('views.musicApplet.back')"
						:title="t('views.musicApplet.back')"
						variant="ghost"
						size="sm"
						@click="pickingOutput = false"
					/>
					<h2 class="min-w-0 truncate text-label-m font-medium text-tx-main">{{ t('views.musicApplet.chooseOutput') }}</h2>
				</div>

				<p v-if="devices.length === 0" class="px-2 text-label-xs text-tx-muted">
					{{ t('views.musicApplet.noOutputs') }}
				</p>
				<div v-else class="min-h-0 overflow-y-auto">
					<OptionGroup
						:model-value="output?.id ?? null"
						:options="outputOptions"
						:label="t('views.musicApplet.chooseOutput')"
						:disabled="switching !== null"
						size="sm"
						@change="onOutputChange"
					/>
				</div>
			</div>
		</div>
	</AppletPopover>
</template>
