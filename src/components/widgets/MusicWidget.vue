<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { SliderControl, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { useMusicPlayer } from '@/tools/composables/useMusicPlayer';
import { formatDuration, sectionsFor } from '@/utils/playback';

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

/** Cuánto dura la pista, y si hay barra que dibujar. */
const hasProgress = computed(() => musicInfo.value.length > 0);
const elapsed = computed(() => formatDuration(position.value));
const total = computed(() => formatDuration(musicInfo.value.length));

/**
 * El volumen del reproductor en 0–100.
 *
 * MPRIS lo publica entre 0 y 1, y el deslizador compartido se mueve de a uno.
 */
const volumePercent = computed({
	get: () => Math.round((musicInfo.value.volume ?? 0) * 100),
	set: (valor: number) => onVolume(Math.min(1, Math.max(0, valor / 100))),
});

const loopStatusIcon = computed(() =>
	musicInfo.value.loopStatus === 'Track' ? loopOneIcon : loopIcon
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

/** El selector de reproductor sólo tiene sentido si hay entre cuáles elegir. */
const showPicker = ref(false);
const canPick = computed(() => players.value.length > 1);

async function togglePicker(): Promise<void> {
	showPicker.value = !showPicker.value;
	if (showPicker.value) await loadPlayers();
}

async function pick(player: string): Promise<void> {
	showPicker.value = false;
	await selectPlayer(player);
}

/** Un clic o un arrastre en la barra: la fracción, y el resto lo hace MPRIS. */
function seekTo(event: Event): void {
	const barra = event.target as HTMLInputElement;
	const largo = musicInfo.value.length;
	if (largo <= 0) return;
	onSeek(Number(barra.value) / largo);
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
	await nextTick();
	updateTitleOverflow();
});

const titleContainer = ref<HTMLElement | null>(null);
const titleInner = ref<HTMLElement | null>(null);
const titleOverflow = ref(false);
const marqueeDistance = ref(0);
const marqueeDuration = ref(6);
/**
 * Si el título no entra, se desliza.
 *
 * Antes esto forzaba el contenedor a 150 px fijos, de cuando el widget tenía un
 * solo tamaño posible. Con la cuadrícula el widget se redimensiona: en uno
 * angosto esa caja se salía de su columna, y en uno ancho recortaba títulos que
 * entraban de sobra. Ahora se mide el ancho que hay y se recalcula cuando el
 * widget cambia de tamaño.
 */
function updateTitleOverflow(): void {
	const container = titleContainer.value;
	const inner = titleInner.value;

	if (!container || !inner) {
		titleOverflow.value = false;
		return;
	}

	const disponible = container.clientWidth;
	const necesario = inner.scrollWidth;

	if (disponible > 0 && necesario > disponible + 2) {
		titleOverflow.value = true;
		marqueeDistance.value = necesario - disponible;
		marqueeDuration.value = Math.min(20, Math.max(4, marqueeDistance.value / 30));
	} else {
		titleOverflow.value = false;
		marqueeDistance.value = 0;
		marqueeDuration.value = 0;
	}
}

let observadorTitulo: ResizeObserver | null = null;

onMounted(() => {
	if (!box.value) return;

	observadorTitulo = new ResizeObserver(() => {
		boxHeight.value = box.value?.clientHeight ?? 0;
		updateTitleOverflow();
	});
	observadorTitulo.observe(box.value);
	boxHeight.value = box.value.clientHeight;
	updateTitleOverflow();
});

onUnmounted(() => observadorTitulo?.disconnect());

watch(
	() => musicInfo.value?.title,
	async () => {
		await nextTick();
		updateTitleOverflow();
	}
);
</script>

<template>
  <!--
    Tres zonas: qué está sonando, dónde va, y qué se puede hacer. Las dos últimas
    aparecen cuando hay alto para ellas — el widget va de una fila a tres y en la
    más chica lo único que no puede faltar es la portada con el transporte.

    Las medidas van en unidades de contenedor —cqmin, el lado más chico— así que
    todo acompaña el tamaño de la celda en los dos ejes. El marco (fondo, blur,
    borde) no está acá: lo pone el contenedor de widgets, igual para todos.
  -->
  <div ref="box" class="relative flex h-full w-full flex-col gap-[2cqmin] p-[3cqmin]">
    <!-- La portada otra vez, ampliada y desenfocada: el widget se tiñe de lo que
         está sonando sin que haya que leerle un color a la imagen. Va detrás de
         todo y no recibe clics. El contenedor ya recorta las esquinas. -->
    <img
      v-if="hasCover"
      :src="imgSrc"
      alt=""
      aria-hidden="true"
      class="pointer-events-none absolute inset-0 h-full w-full scale-150 object-cover opacity-30 blur-2xl saturate-150"
    />

    <!-- Arriba: la portada y el título. `min-h-0` para que esta fila pueda
         encogerse y los controles nunca queden fuera del widget. -->
    <div class="relative flex min-h-0 flex-1 items-center gap-[3cqmin]">
      <button
        v-if="musicInfo.canRaise"
        type="button"
        class="aspect-square h-full max-h-full shrink-0 overflow-hidden rounded-corner"
        :title="t('components.MusicWidget.raise')"
        :aria-label="t('components.MusicWidget.raise')"
        @click.prevent="onRaise"
      >
        <img
          :src="imgSrc"
          :alt="musicInfo.title"
          class="h-full w-full object-cover"
          @error="onImgError"
        />
      </button>
      <img
        v-else
        :src="imgSrc"
        :alt="musicInfo.title"
        :title="musicInfo.title"
        class="aspect-square h-full max-h-full shrink-0 rounded-corner object-cover"
        @error="onImgError"
      />

      <div class="flex min-w-0 flex-1 flex-col justify-center">
        <div ref="titleContainer" class="overflow-hidden" :title="musicInfo.title || ''">
          <span
            ref="titleInner"
            class="inline-block whitespace-nowrap text-[clamp(0.72rem,13cqmin,1.05rem)] font-medium text-tx-main"
            :class="{ marquee: titleOverflow }"
            :style="
              titleOverflow
                ? {
                    '--marquee-distance': `${marqueeDistance}px`,
                    '--marquee-duration': `${marqueeDuration}s`,
                  }
                : {}
            "
          >
            {{ musicInfo.title || t('components.MusicWidget.nothingPlaying') }}
          </span>
        </div>

        <div
          class="truncate text-[clamp(0.65rem,10cqmin,0.9rem)] text-tx-muted"
          :title="musicInfo.artist || ''"
        >
          {{ musicInfo.artist || "" }}
        </div>

        <div
          v-if="sections.album && musicInfo.album"
          class="truncate text-[clamp(0.6rem,8cqmin,0.8rem)] text-tx-muted opacity-80"
          :title="musicInfo.album"
        >
          {{ musicInfo.album }}
        </div>
      </div>

      <!-- Qué reproductor se está siguiendo, y a cuál cambiar. Aparece si hay
           más de uno —con uno solo no hay nada que elegir— y si el widget tiene
           alto: en el de una fila se le comía el título. -->
      <div v-if="canPick && sections.album" class="relative shrink-0">
        <button
          type="button"
          class="max-w-[25cqw] truncate rounded-corner bg-ui-surface/60 px-[2cqmin] py-[1cqmin] text-[clamp(0.55rem,7cqmin,0.72rem)] text-tx-muted transition-colors hover:bg-ui-surface"
          :title="t('components.MusicWidget.choosePlayer')"
          :aria-label="t('components.MusicWidget.choosePlayer')"
          :aria-expanded="showPicker"
          @click.prevent="togglePicker"
        >
          {{ musicInfo.playerIdentity || t('components.MusicWidget.choosePlayer') }}
        </button>

        <div
          v-if="showPicker"
          class="absolute right-0 top-full z-10 mt-1 flex min-w-[28cqmin] flex-col gap-1 rounded-corner border border-ui-border bg-ui-surface/95 p-1 backdrop-blur-md"
        >
          <button
            v-for="p in players"
            :key="p.player"
            type="button"
            class="truncate rounded-corner px-2 py-1 text-left text-xs transition-colors hover:bg-primary/40"
            :class="p.pinned || p.active ? 'text-tx-main' : 'text-tx-muted'"
            @click.prevent="pick(p.player)"
          >
            {{ p.identity }}<span v-if="p.title"> — {{ p.title }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Dónde va la pista. Sin duración publicada no hay barra: una radio en
         vivo no sabe cuánto dura, y una barra ahí diría algo que nadie sabe. -->
    <div
      v-if="sections.progress && hasProgress"
      class="relative flex shrink-0 items-center gap-[2cqmin] text-[clamp(0.55rem,7cqmin,0.72rem)] text-tx-muted tabular-nums"
    >
      <span>{{ elapsed }}</span>
      <input
        type="range"
        class="h-1 min-w-0 flex-1 accent-primary disabled:opacity-50"
        min="0"
        :max="musicInfo.length"
        :value="position"
        :disabled="!musicInfo.canSeek"
        :aria-label="t('components.MusicWidget.seek')"
        :aria-valuetext="`${elapsed} / ${total}`"
        @change="seekTo"
      />
      <span>{{ total }}</span>
    </div>

    <!-- El transporte. Cada botón dice si sirve: con un vídeo de YouTube el
         navegador contesta que no se puede ir al anterior ni al siguiente, y
         antes los dos se dibujaban igual y no pasaba nada al apretarlos. -->
    <div class="relative flex shrink-0 items-stretch gap-[2cqmin]">
      <button
        v-if="sections.extras && musicInfo.shuffle !== null"
        type="button"
        class="flex h-[clamp(1.25rem,20cqmin,2.5rem)] w-[clamp(1.25rem,20cqmin,2.5rem)] shrink-0 items-center justify-center rounded-corner transition-colors"
        :class="musicInfo.shuffle ? 'bg-primary/80 hover:bg-primary' : 'bg-ui-surface/60 hover:bg-ui-surface'"
        :title="t('components.MusicWidget.shuffle')"
        :aria-label="t('components.MusicWidget.shuffle')"
        :aria-pressed="musicInfo.shuffle"
        @click.prevent="onShuffle"
      >
        <ThemeIcon :name="shuffleIcon" type="symbol" :size="16" :alt="t('components.MusicWidget.shuffle')" />
      </button>

      <button
        type="button"
        class="flex h-[clamp(1.25rem,20cqmin,2.5rem)] flex-1 items-center justify-center rounded-corner bg-ui-surface/60 transition-colors hover:bg-ui-surface disabled:cursor-default disabled:opacity-40 disabled:hover:bg-ui-surface/60"
        :title="t('components.MusicWidget.previous')"
        :aria-label="t('components.MusicWidget.previous')"
        :disabled="!musicInfo.canGoPrevious"
        @click.prevent="onPrev"
      >
        <ThemeIcon :name="prevIcon" type="symbol" :size="20" :alt="t('components.MusicWidget.previous')" />
      </button>

      <button
        type="button"
        class="flex h-[clamp(1.25rem,20cqmin,2.5rem)] flex-[1.4] items-center justify-center rounded-corner bg-primary/80 transition-colors hover:bg-primary"
        :title="isPlaying ? t('components.MusicWidget.pause') : t('components.MusicWidget.play')"
        :aria-label="isPlaying ? t('components.MusicWidget.pause') : t('components.MusicWidget.play')"
        @click.prevent="onPlayPause"
      >
        <ThemeIcon
          :name="isPlaying ? pauseIcon : playIcon"
          type="symbol"
          :size="22"
          :alt="isPlaying ? t('components.MusicWidget.pause') : t('components.MusicWidget.play')"
        />
      </button>

      <button
        type="button"
        class="flex h-[clamp(1.25rem,20cqmin,2.5rem)] flex-1 items-center justify-center rounded-corner bg-ui-surface/60 transition-colors hover:bg-ui-surface disabled:cursor-default disabled:opacity-40 disabled:hover:bg-ui-surface/60"
        :title="t('components.MusicWidget.next')"
        :aria-label="t('components.MusicWidget.next')"
        :disabled="!musicInfo.canGoNext"
        @click.prevent="onNext"
      >
        <ThemeIcon :name="nextIcon" type="symbol" :size="20" :alt="t('components.MusicWidget.next')" />
      </button>

      <button
        v-if="sections.extras && musicInfo.loopStatus !== null"
        type="button"
        class="flex h-[clamp(1.25rem,20cqmin,2.5rem)] w-[clamp(1.25rem,20cqmin,2.5rem)] shrink-0 items-center justify-center rounded-corner transition-colors"
        :class="musicInfo.loopStatus !== 'None' ? 'bg-primary/80 hover:bg-primary' : 'bg-ui-surface/60 hover:bg-ui-surface'"
        :title="loopLabel"
        :aria-label="loopLabel"
        @click.prevent="onLoop"
      >
        <ThemeIcon :name="loopStatusIcon" type="symbol" :size="16" :alt="loopLabel" />
      </button>

      <button
        v-if="sections.extras"
        type="button"
        class="flex h-[clamp(1.25rem,20cqmin,2.5rem)] w-[clamp(1.25rem,20cqmin,2.5rem)] shrink-0 items-center justify-center rounded-corner bg-ui-surface/60 transition-colors hover:bg-ui-surface disabled:cursor-default disabled:opacity-40"
        :title="t('components.MusicWidget.stop')"
        :aria-label="t('components.MusicWidget.stop')"
        :disabled="!musicInfo.canControl"
        @click.prevent="onStop"
      >
        <ThemeIcon :name="stopIcon" type="symbol" :size="18" :alt="t('components.MusicWidget.stop')" />
      </button>
    </div>

    <!-- Aleatorio, repetición y volumen: los tres existen sólo si el reproductor
         los implementa. `null` es «no los tiene» y ahí no se dibujan; `false` es
         «los tiene y están apagados», que es otra cosa. -->
    <div
      v-if="sections.extras && musicInfo.volume !== null"
      class="relative flex shrink-0 items-center gap-[2cqmin]"
    >
      <div class="min-w-0 flex-1">
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

    <!-- Los avisos van superpuestos abajo: si empujaran el layout, un error
         haría saltar la portada y los botones. -->
    <div class="pointer-events-none absolute inset-x-[4cqmin] bottom-[3cqmin] flex flex-col gap-1">
      <transition
        enter-active-class="transition-all duration-300 ease-out"
        leave-active-class="transition-all duration-300 ease-out"
        enter-from-class="opacity-0 -translate-y-1"
        leave-to-class="opacity-0 translate-y-1"
      >
        <div
          v-if="dbusStatus === 'reconnecting' || dbusStatus === 'failed'"
          class="rounded-corner px-2 py-1 text-xs text-ui-main"
          :class="dbusStatus === 'reconnecting' ? 'bg-status-warning' : 'bg-status-error'"
        >
          {{ dbusMessage }}
        </div>
      </transition>
    </div>
  </div>
</template>

<style scoped>
/**
 * El deslizamiento del título, que hasta ahora no existía.
 *
 * La clase estaba puesta desde siempre y las variables se calculaban, pero la
 * animación no estaba escrita en ninguna parte: el título largo se cortaba y ya.
 * Nada fallaba, que es por qué sobrevivió tanto.
 */
.marquee {
  animation: vsk-marquee var(--marquee-duration, 6s) ease-in-out infinite alternate;
}

@keyframes vsk-marquee {
  from {
    transform: translateX(0);
  }
  to {
    transform: translateX(calc(-1 * var(--marquee-distance, 0px)));
  }
}

/* Quien pidió que el escritorio no se mueva no pidió una excepción para esto. */
@media (prefers-reduced-motion: reduce) {
  .marquee {
    animation: none;
  }
}
</style>
