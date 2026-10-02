<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	Chip,
	NowPlayingCard,
	OptionGroup,
	type OptionGroupOption,
	PageDots,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref, watch } from 'vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
import type { AudioDevice } from '@/interfaces/audio-device';
import { getAudioDevices, setAudioDevice } from '@/services/core.service';
import { currentOutput, outputIcon, outputLabel } from '@/tools/audio-outputs';
import { useMusicPlayer } from '@/tools/composables/useMusicPlayer';
import { useSharedEvent } from '@/tools/event.bus';
import { activePlayerIndex, playerLabels } from '@/tools/music-players';
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
 * abiertos, unos puntos al pie (`PageDots`) pasan de uno a otro: tocar uno lo
 * fija con `music_select_player`, el mismo camino que el selector del widget.
 * Con uno solo los puntos no se dibujan.
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
	players,
	loadPlayers,
	selectPlayer,
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

/** Por dónde suena: el nombre del reproductor, con «vía» delante en la pastilla. */
const via = computed(() => musicInfo.value.playerIdentity || '');

// ── Los reproductores ───────────────────────────────────────────────────────

const activePlayer = computed(() => activePlayerIndex(players.value, musicInfo.value.player));
const dotLabels = computed(() => playerLabels(players.value));

function onPlayerChange(index: number): void {
	const entry = players.value[index];
	if (entry) void selectPlayer(entry.player);
}

// Otro reproductor pasó a ser el que suena —se abrió uno, se cerró el que
// estaba—: la lista que se muestra puede haber cambiado.
watch(
	() => musicInfo.value.player,
	() => {
		void loadPlayers();
	}
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
	await Promise.all([initMusicInfo(), loadDevices(), loadPlayers()]);
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

/**
 * La salida marcada, controlada por la que el sistema confirma: `OptionGroup`
 * mueve su marca antes de avisar, y si el cambio falla, sin esto la salida
 * que no se pudo poner quedaba elegida y volver a tocarla no hacía nada.
 */
const checkedOutput = computed({
	get: () => output.value?.id ?? null,
	set: () => {},
});

function onOutputChange(id: string): void {
	const device = devices.value.find((candidate) => candidate.id === id);
	if (device) void chooseOutput(device);
}
</script>

<template>
	<AppletPopover applet="music" @shown="refresh">
		<!-- La tarjeta o el selector de salida, uno por vez: el applet mide lo
		     que mide, y una lista que empujara la tarjeta la sacaría de la
		     ventana. El selector **reemplaza** a la tarjeta, como una ficha con
		     volver, en vez de taparla con una capa opaca: así el applet sigue
		     dejando ver el desenfoque de Wayfire. El alto es el de `APPLETS`, el
		     del caso más alto (título en dos renglones y los puntos de varios
		     reproductores); lo que sobra en los demás se reparte arriba y abajo
		     en vez de quedar como un hueco al pie. En un monitor tan angosto que
		     el applet no entra en su ancho, la tarjeta pasa a una columna (lo
		     hace la librería por container query), las pastillas se parten en
		     renglones y lo que no entra en el alto se desplaza: nada se corta. -->
		<div class="@container flex h-full min-h-0 flex-col justify-center-safe overflow-y-auto" data-music-applet>
			<template v-if="!pickingOutput">
				<NowPlayingCard
					:title="musicInfo.title"
					:artist="musicInfo.artist"
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
						<!-- Dos pastillas en un renglón, como en la referencia: la
						     salida, que abre el selector, y la aplicación de origen,
						     que sólo informa. No se parten: cada una se corta con
						     puntos suspensivos y el nombre entero queda en el globo.
						     Así la tarjeta mide siempre lo mismo y entra en el alto
						     de `APPLETS`. -->
						<div class="flex w-full min-w-0 flex-wrap justify-center gap-1 @xs:flex-nowrap @xs:justify-start" data-chips>
							<Chip
								v-if="output"
								interactive
								class="min-w-0 shrink"
								:label="outputLabel(output)"
								:icon="outputIcon(output)"
								:title="t('views.musicApplet.chooseOutput')"
								:aria-label="outputAnnouncement"
								:aria-expanded="pickingOutput"
								@click="pickingOutput = true"
							/>
							<!-- Sin icono, como el «VIA Firefox» de la referencia: el
							     icono de la aplicación ya está en el disco cuando no hay
							     carátula. Hasta la mitad del renglón, para que un nombre
							     largo («Reproductor multimedia VLC») no deje la salida en
							     tres letras, ni al revés. -->
							<Chip
								v-if="via"
								class="max-w-1/2 shrink"
								:caption="t('views.musicApplet.viaCaption')"
								:label="via"
								:title="`${t('views.musicApplet.viaCaption')} ${via}`"
							/>
						</div>
					</template>
				</NowPlayingCard>

				<!-- Con más de un reproductor, un punto por cada uno; con uno
				     solo no se dibuja nada, ni el hueco. -->
				<PageDots
					class="mt-2"
					:count="players.length"
					:model-value="activePlayer"
					:labels="dotLabels"
					:label="t('views.musicApplet.players')"
					@change="onPlayerChange"
				/>
			</template>

			<!-- El selector de salida: `OptionGroup` de la librería, el mismo del
			     applet de audio, un `radiogroup` con las flechas y un solo Tab. -->
			<div v-else class="flex min-h-0 flex-1 flex-col gap-2" data-output-picker>
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
				<div v-else class="min-h-0 overflow-y-auto rounded-corner-m bg-ui-surface/70 p-2">
					<OptionGroup
						v-model="checkedOutput"
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
