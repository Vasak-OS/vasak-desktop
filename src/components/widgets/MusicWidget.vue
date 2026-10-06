<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	Chip,
	PageDots,
	SeekBar,
	SliderControl,
	SpinningCover,
	ToggleControl,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useMusicPlayer } from '@/tools/composables/useMusicPlayer';
import { activePlayerIndex, playerLabels } from '@/tools/music-players';
import {
	coverRingProgress,
	hasExtras as extrasOf,
	loopStatusIconName,
	percentToVolume,
	playPauseAvailable,
	seekRatio,
	volumeToPercent,
} from '@/tools/music-widget';
import { formatDuration, playbackStateOf, sectionsFor } from '@/utils/playback';

/**
 * El widget de música del escritorio, con la estética del reproductor nuevo del
 * panel (vasak-desktop#166).
 *
 * Usa las mismas piezas de vue-libvasak que arma la `NowPlayingCard` del applet
 * (`MusicAppletView`): el disco que gira (`SpinningCover`, con el aro de avance),
 * la barra (`SeekBar`), y los controles de la librería —no dibujados a mano
 * (decisión 8)—. No es la tarjeta entera porque el widget por omisión es una
 * **fila** (4×1, 120 px de alto) y la tarjeta no entra ahí; la forma del widget
 * no cambia (decisión 3): se arma con las mismas piezas y así se ve igual. Según
 * el alto de la celda van apareciendo la barra, el álbum y los extras. Los datos
 * son los de `useMusicPlayer`, el mismo camino a MPRIS que el panel y el applet.
 */

const { t } = useI18n();

const {
	musicInfo,
	imgSrc,
	hasCover,
	players,
	isPlaying,
	position,
	prevIcon,
	nextIcon,
	playIcon,
	pauseIcon,
	stopIcon,
	shuffleIcon,
	loopIcon,
	loopOneIcon,
	volumeIcon,
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
} = useMusicPlayer();

const dbusStatus = ref('connected');
const dbusMessage = ref('');

/**
 * El tamaño del widget, medido.
 *
 * En el WebView no llegan ni `matchMedia` ni el `resize` de la ventana, así que
 * lo que hay que mirar es la caja propia. De este alto sale qué secciones entran.
 */
const box = ref<HTMLElement | null>(null);
const boxHeight = ref(0);
const sections = computed(() => sectionsFor(boxHeight.value));

/** El estado del disco: suena, pausa o detenido. */
const state = computed(() => playbackStateOf(musicInfo.value.status));
/** El aro de avance del disco, de 0 a 100; sin duración conocida, sin aro. */
const ringProgress = computed(() => coverRingProgress(position.value, musicInfo.value.length));

/** Reproducir o pausar, según lo que el reproductor diga que acepta ahora. */
const canPlayPause = computed(() => playPauseAvailable(musicInfo.value, isPlaying.value));

/** Por dónde suena: el nombre del reproductor, con «vía» delante en la pastilla. */
const via = computed(() => musicInfo.value.playerIdentity || '');

/** La barra emite en microsegundos; el composable salta por fracción. */
function seekTo(micros: number): void {
	const ratio = seekRatio(micros, musicInfo.value.length);
	if (ratio !== null) onSeek(ratio);
}

/**
 * El volumen del reproductor en 0–100.
 *
 * MPRIS lo publica entre 0 y 1, y el deslizador compartido se mueve de a uno.
 */
const volumePercent = computed({
	get: () => volumeToPercent(musicInfo.value.volume),
	set: (value: number) => onVolume(percentToVolume(value)),
});

const loopStatusIcon = computed(() =>
	loopStatusIconName(musicInfo.value.loopStatus, loopIcon, loopOneIcon)
);
const loopLabel = computed(() => {
	switch (musicInfo.value.loopStatus) {
		case 'Track':
			return t('components.MusicWidget.loopTrack');
		case 'Playlist':
			return t('components.MusicWidget.loopPlaylist');
		default:
			return t('components.MusicWidget.loopOff');
	}
});

/** Sin duración publicada no hay barra: una radio en vivo no sabe cuánto dura. */
const hasProgress = computed(() => musicInfo.value.length > 0);
/** Si hay aleatorio, repetición o volumen que mostrar. */
const hasExtras = computed(() => extrasOf(musicInfo.value));

// ── Los reproductores ─────────────────────────────────────────────────────────

/** El selector sólo tiene sentido con más de uno entre cuáles elegir. */
const canPick = computed(() => players.value.length > 1);
const activePlayer = computed(() => activePlayerIndex(players.value, musicInfo.value.player));
const dotLabels = computed(() => playerLabels(players.value));

function onPlayerChange(index: number): void {
	const entry = players.value[index];
	if (entry) void selectPlayer(entry.player);
}

onMounted(async () => {
	await initIcons();
	await initMusicInfo();
	await loadPlayers();
	listen('dbus-status', (event: any) => {
		const payload = event.payload;
		if (payload.service === 'music') {
			dbusStatus.value = payload.status;
			if (payload.status === 'reconnecting') {
				dbusMessage.value = t('components.MusicWidget.reconnecting').replace(
					'{0}',
					String(payload.attempt)
				);
			} else if (payload.status === 'failed') {
				dbusMessage.value = payload.message || t('components.MusicWidget.connectionError');
			} else if (payload.status === 'connected') {
				dbusMessage.value = '';
			}
		}
	});
});

let boxObserver: ResizeObserver | null = null;
onMounted(() => {
	if (!box.value) return;
	boxObserver = new ResizeObserver(() => {
		boxHeight.value = box.value?.clientHeight ?? 0;
	});
	boxObserver.observe(box.value);
	boxHeight.value = box.value.clientHeight;
});
onUnmounted(() => boxObserver?.disconnect());

/**
 * La lista se vuelve a pedir cuando cambia el reproductor que suena: si el
 * segundo arranca después, el selector tiene que aparecer igual.
 */
watch(() => musicInfo.value?.player, loadPlayers);
</script>

<template>
  <!--
    Las mismas piezas que la tarjeta del panel, en la forma del widget. El marco
    (fondo, blur, borde) lo pone el contenedor de widgets, igual para todos. Las
    medidas van en unidades de contenedor —cqmin, el lado más chico— así que todo
    acompaña el tamaño de la celda. Las secciones de abajo aparecen según el alto.
  -->
  <div ref="box" class="relative flex h-full w-full flex-col justify-center gap-[2cqmin] overflow-hidden p-[3cqmin]">
    <!-- La portada ampliada y desenfocada de fondo: el widget se tiñe de lo que
         suena. Va detrás de todo y no recibe clics; el contenedor recorta las
         esquinas. -->
    <img
      v-if="hasCover"
      :src="imgSrc"
      alt=""
      aria-hidden="true"
      class="pointer-events-none absolute inset-0 h-full w-full scale-150 object-cover opacity-30 blur-2xl saturate-150"
    />

    <!-- Arriba: el disco que gira y el título. `min-h-0` para que esta fila se
         pueda encoger y el transporte nunca quede fuera del widget. -->
    <div class="relative flex min-h-0 flex-1 items-center gap-[3cqmin]">
      <!-- El disco ocupa el alto de la fila, cuadrado, con un tope para que en
           una celda grande no se agigante. -->
      <div class="aspect-square shrink-0" style="height: clamp(2.5rem, 100%, 6rem)">
        <SpinningCover
          class="h-full w-full"
          :src="imgSrc"
          :alt="musicInfo.title"
          :state="state"
          :progress="ringProgress"
          :progress-label="t('components.MusicWidget.seek')"
          :interactive="musicInfo.canRaise"
          :label="t('components.MusicWidget.raise')"
          @click="onRaise"
          @error="onImgError"
        />
      </div>

      <div class="flex min-w-0 flex-1 flex-col justify-center">
        <div
          class="truncate text-[clamp(0.72rem,13cqmin,1.05rem)] font-semibold text-tx-main"
          :title="musicInfo.title || ''"
        >
          {{ musicInfo.title || t('components.MusicWidget.nothingPlaying') }}
        </div>
        <div
          v-if="musicInfo.artist"
          class="truncate text-[clamp(0.65rem,10cqmin,0.9rem)] text-tx-muted"
          :title="musicInfo.artist"
        >
          {{ musicInfo.artist }}
        </div>
        <div
          v-if="sections.album && musicInfo.album"
          class="truncate text-[clamp(0.6rem,8cqmin,0.8rem)] text-tx-muted opacity-80"
          :title="musicInfo.album"
        >
          {{ musicInfo.album }}
        </div>

        <!-- Quién suena, como la pastilla «vía» de la tarjeta del panel. Sólo
             cuando hay alto: en la fila baja se come el título. -->
        <div v-if="sections.album && via" class="mt-[1cqmin] flex min-w-0">
          <Chip
            class="min-w-0 shrink"
            :caption="t('components.MusicWidget.viaCaption')"
            :label="via"
            :title="`${t('components.MusicWidget.viaCaption')} ${via}`"
          />
        </div>
      </div>

      <!-- El transporte, con los botones de la librería: anterior, reproducir y
           siguiente, como en la tarjeta. Al costado del título aprovecha el
           ancho de la fila; cada botón dice si sirve. -->
      <div class="flex shrink-0 items-center gap-[2cqmin]">
        <ActionButton
          label=""
          :icon="prevIcon"
          icon-type="symbol"
          :icon-alt="t('components.MusicWidget.previous')"
          :title="t('components.MusicWidget.previous')"
          variant="ghost"
          size="sm"
          :disabled="!musicInfo.canGoPrevious"
          @click="onPrev"
        />
        <ActionButton
          label=""
          :icon="isPlaying ? pauseIcon : playIcon"
          icon-type="symbol"
          :icon-alt="isPlaying ? t('components.MusicWidget.pause') : t('components.MusicWidget.play')"
          :title="isPlaying ? t('components.MusicWidget.pause') : t('components.MusicWidget.play')"
          variant="primary"
          :disabled="!canPlayPause"
          @click="onPlayPause"
        />
        <ActionButton
          label=""
          :icon="nextIcon"
          icon-type="symbol"
          :icon-alt="t('components.MusicWidget.next')"
          :title="t('components.MusicWidget.next')"
          variant="ghost"
          size="sm"
          :disabled="!musicInfo.canGoNext"
          @click="onNext"
        />
      </div>
    </div>

    <!-- Dónde va la pista: la barra de la librería, la misma de la tarjeta. -->
    <SeekBar
      v-if="sections.progress && hasProgress"
      class="relative shrink-0"
      :position="position"
      :duration="musicInfo.length"
      :seekable="musicInfo.canSeek"
      :step="5_000_000"
      :format="formatDuration"
      :label="t('components.MusicWidget.seek')"
      @seek="seekTo"
    />

    <!-- Aleatorio, repetición, parar y volumen: cada uno existe sólo si el
         reproductor lo implementa (`null` es «no lo tiene»). Aparecen cuando la
         celda tiene alto para ellos. -->
    <div
      v-if="sections.extras && hasExtras"
      class="relative flex shrink-0 items-center gap-[2cqmin]"
    >
      <ToggleControl
        v-if="musicInfo.shuffle !== null"
        :name="shuffleIcon"
        type="symbol"
        :label="t('components.MusicWidget.shuffle')"
        :pressed="musicInfo.shuffle"
        @click="onShuffle"
      />
      <ToggleControl
        v-if="musicInfo.loopStatus !== null"
        :name="loopStatusIcon"
        type="symbol"
        :label="loopLabel"
        :pressed="musicInfo.loopStatus !== 'None'"
        @click="onLoop"
      />
      <ActionButton
        label=""
        :icon="stopIcon"
        icon-type="symbol"
        :icon-alt="t('components.MusicWidget.stop')"
        :title="t('components.MusicWidget.stop')"
        variant="ghost"
        size="sm"
        :disabled="!musicInfo.canControl"
        @click="onStop"
      />
      <div v-if="musicInfo.volume !== null" class="min-w-0 flex-1">
        <SliderControl
          v-model="volumePercent"
          :name="volumeIcon"
          type="symbol"
          :label="t('components.MusicWidget.volume')"
          :min="0"
          :max="100"
        />
      </div>
    </div>

    <!-- Con más de un reproductor, un punto por cada uno; con uno solo nada.
         Igual que el applet del panel. -->
    <PageDots
      v-if="canPick && sections.album"
      class="relative shrink-0"
      :count="players.length"
      :model-value="activePlayer"
      :labels="dotLabels"
      :label="t('components.MusicWidget.players')"
      @change="onPlayerChange"
    />

    <!-- Los avisos van superpuestos abajo: si empujaran el layout, un error
         haría saltar el disco y los botones. -->
    <div class="pointer-events-none absolute inset-x-[4cqmin] bottom-[3cqmin] flex flex-col gap-1">
      <transition
        enter-active-class="transition-all duration-300 ease-out"
        leave-active-class="transition-all duration-300 ease-out"
        enter-from-class="opacity-0 -translate-y-1"
        leave-to-class="opacity-0 translate-y-1"
      >
        <div
          v-if="dbusStatus === 'reconnecting' || dbusStatus === 'failed'"
          class="rounded-corner-m px-2 py-1 text-xs text-tx-main"
          :class="dbusStatus === 'reconnecting' ? 'bg-status-warning' : 'bg-status-error'"
        >
          {{ dbusMessage }}
        </div>
      </transition>
    </div>
  </div>
</template>
