<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton, SpinningCover } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { toggleApplet } from '@/services/window.service';
import { useMusicPlayer } from '@/tools/composables/useMusicPlayer';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
import { logError } from '@/utils/logger';
import { formatDuration, playbackStateOf } from '@/utils/playback';

const { t } = useI18n();

const {
	musicInfo,
	imgSrc,
	position,
	progress,
	prevIcon,
	nextIcon,
	playIcon,
	pauseIcon,
	isPlaying,
	onPrev,
	onNext,
	onPlayPause,
	onImgError,
	initIcons,
	initMusicInfo,
} = useMusicPlayer();

const visible = ref(false);
const isHiding = ref(false);
let hideTimer: ReturnType<typeof setTimeout> | null = null;
const ANIM_MS = 180;

/**
 * Lo que dice el globo: qué suena, de quién, de qué disco y por dónde va.
 *
 * Es la única parte de la bandeja donde entra texto: la fila del panel mide 22
 * píxeles y el resto son iconos. Antes decía sólo el título.
 */
const summary = computed(() => {
	const info = musicInfo.value;
	if (!info.title) return t('components.TrayMusicControl.nothingPlaying');

	const lines = [info.title];
	if (info.artist) lines.push(info.artist);
	if (info.album) lines.push(info.album);
	if (info.length > 0) {
		lines.push(`${formatDuration(position.value)} / ${formatDuration(info.length)}`);
	}
	return lines.join('\n');
});

/** Si suena, está en pausa o no hay nada: el disco gira, se congela o se queda quieto. */
const state = computed(() => playbackStateOf(musicInfo.value.status));

/**
 * El reproductor desplegable, colgado de este control.
 *
 * Lo abre la portada —que es lo que se ve siempre, con o sin el mouse encima—
 * y no los botones del transporte, que siguen haciendo lo suyo sin abrir nada.
 */
const opener = ref<HTMLElement | null>(null);
const { openClasses } = useOpenApplet('music');

async function openPlayer(): Promise<void> {
	try {
		await toggleApplet('music', opener.value);
	} catch (error) {
		logError('[TrayMusicControl] no se pudo abrir el reproductor:', error);
	}
}

function onEnter(): void {
	if (hideTimer) {
		clearTimeout(hideTimer);
		hideTimer = null;
	}
	isHiding.value = false;
	visible.value = true;
}

function onLeave(): void {
	if (!visible.value) return;
	isHiding.value = true;
	if (hideTimer) clearTimeout(hideTimer);
	hideTimer = setTimeout(() => {
		visible.value = false;
		isHiding.value = false;
		hideTimer = null;
	}, ANIM_MS);
}

onMounted(async () => {
	await initIcons();
	await initMusicInfo();
});

onUnmounted(() => {
	if (hideTimer) clearTimeout(hideTimer);
});
</script>

<template>
  <!-- contenedor con handlers para controlar la visibilidad -->
  <div
    class="p-1 rounded-corner-m hover:bg-ui-hover flex items-center"
    :class="openClasses"
    @mouseenter="onEnter"
    @mouseleave="onLeave"
  >
    <!-- La portada, que gira mientras suena, con el aro de progreso alrededor:
         es `SpinningCover` de la librería (2.2.0), que se congela en pausa en
         vez de volver a cero, se queda quieta con `motion-reduce`, y con
         `interactive` es un botón con nombre que suma el avance. El aro dice
         por dónde va sin ocupar una fila más, que en 22 píxeles de panel no
         existe. El envoltorio es el ancla del reproductor desplegable. -->
    <span ref="opener" class="flex shrink-0" :title="summary">
      <SpinningCover
        class="size-5.5"
        :src="imgSrc"
        :alt="musicInfo.title"
        :state="state"
        :progress="musicInfo.length > 0 ? progress * 100 : null"
        :progress-label="t('components.TrayMusicControl.progress')"
        interactive
        :label="t('components.TrayMusicControl.openPlayer')"
        @click="openPlayer"
        @error="onImgError"
      />
    </span>

    <div
      v-show="visible || isHiding"
      :class="[
        ' ml-2 flex items-center pr-1 space-x-1 transition-all duration-150',
        visible && !isHiding ? 'controls-anim-in' : '',
        isHiding ? 'controls-anim-out' : '',
      ]"
      :style="{
        pointerEvents: visible || isHiding ? 'auto' : 'none',
        display: visible || isHiding ? 'flex' : 'none',
      }"
      aria-hidden="false"
    >
      <!-- Qué está sonando, que hasta ahora sólo estaba en el globo. -->
      <span
        v-if="musicInfo.title"
        class="max-w-40 truncate text-label-xs text-tx-main"
        :title="summary"
      >
        {{ musicInfo.title }}<span v-if="musicInfo.artist" class="text-tx-muted"> — {{ musicInfo.artist }}</span>
      </span>

      <!-- Los tres de transporte son el botón `ghost` de la librería en su
           tamaño chico: se ve de 24, como antes, y se apunta en 32. -->
      <ActionButton
        label=""
        :icon="prevIcon"
        :icon-alt="t('components.TrayMusicControl.previous')"
        :title="t('components.TrayMusicControl.previous')"
        variant="ghost"
        size="sm"
        prevent-default
        :disabled="!musicInfo.canGoPrevious"
        @click="onPrev"
      />

      <ActionButton
        label=""
        :icon="isPlaying ? pauseIcon : playIcon"
        :icon-alt="isPlaying
          ? t('components.TrayMusicControl.pause')
          : t('components.TrayMusicControl.play')"
        :title="isPlaying
          ? t('components.TrayMusicControl.pause')
          : t('components.TrayMusicControl.play')"
        variant="ghost"
        size="sm"
        prevent-default
        @click="onPlayPause"
      />

      <ActionButton
        label=""
        :icon="nextIcon"
        :icon-alt="t('components.TrayMusicControl.next')"
        :title="t('components.TrayMusicControl.next')"
        variant="ghost"
        size="sm"
        prevent-default
        :disabled="!musicInfo.canGoNext"
        @click="onNext"
      />
    </div>
  </div>
</template>
