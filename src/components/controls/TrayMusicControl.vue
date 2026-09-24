<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useMusicPlayer } from '@/tools/composables/useMusicPlayer';
import { formatDuration } from '@/utils/playback';

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
const resumen = computed(() => {
	const info = musicInfo.value;
	if (!info.title) return t('components.TrayMusicControl.nothingPlaying');

	const lineas = [info.title];
	if (info.artist) lineas.push(info.artist);
	if (info.album) lineas.push(info.album);
	if (info.length > 0) {
		lineas.push(`${formatDuration(position.value)} / ${formatDuration(info.length)}`);
	}
	return lineas.join('\n');
});

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
    class="p-1 rounded-corner hover:bg-primary flex items-center"
    @mouseenter="onEnter"
    @mouseleave="onLeave"
  >
    <!-- La portada, que gira mientras suena. `motion-reduce` la deja quieta:
         quien pidió que el escritorio no se mueva no pidió una excepción para
         la bandeja. Debajo, el aro de progreso dice por dónde va sin ocupar
         una fila más, que en 22 píxeles de panel no existe. -->
    <div class="relative w-5.5 h-5.5 shrink-0">
      <img
        :src="imgSrc"
        :alt="musicInfo.title"
        :title="resumen"
        class="w-full h-full rounded-full origin-center object-cover"
        :class="{ 'animate-spin motion-reduce:animate-none': isPlaying }"
        @error="onImgError"
      />
      <div
        v-if="musicInfo.length > 0"
        class="pointer-events-none absolute inset-0 rounded-full"
        :style="{
          background: `conic-gradient(var(--color-primary) ${progress * 360}deg, transparent 0deg)`,
          mask: 'radial-gradient(circle, transparent 72%, black 74%)',
          WebkitMask: 'radial-gradient(circle, transparent 72%, black 74%)',
        }"
        aria-hidden="true"
      ></div>
    </div>

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
        class="max-w-40 truncate text-xs text-tx-main"
        :title="resumen"
      >
        {{ musicInfo.title }}<span v-if="musicInfo.artist" class="text-tx-muted"> — {{ musicInfo.artist }}</span>
      </span>

      <button
        type="button"
        @click.prevent="onPrev"
        :disabled="!musicInfo.canGoPrevious"
        class="w-6 h-6 flex items-center justify-center rounded-corner bg-ui-bg/80 text-xs disabled:cursor-default disabled:opacity-40"
        :title="t('components.TrayMusicControl.previous')" :aria-label="t('components.TrayMusicControl.previous')">
        <ThemeIcon :name="prevIcon" type="symbol" :size="16" :alt="t('components.TrayMusicControl.previous')" />
      </button>

      <button
        type="button"
        @click.prevent="onPlayPause"
        class="w-6 h-6 flex items-center justify-center rounded-corner bg-ui-bg/80 text-xs"
        :title="isPlaying
          ? t('components.TrayMusicControl.pause')
          : t('components.TrayMusicControl.play')" :aria-label="isPlaying
          ? t('components.TrayMusicControl.pause')
          : t('components.TrayMusicControl.play')">
        <ThemeIcon
          :name="isPlaying ? pauseIcon : playIcon"
          type="symbol"
          :size="16"
          :alt="isPlaying
            ? t('components.TrayMusicControl.pause')
            : t('components.TrayMusicControl.play')"
        />
      </button>

      <button
        type="button"
        @click.prevent="onNext"
        :disabled="!musicInfo.canGoNext"
        class="w-6 h-6 flex items-center justify-center rounded-corner bg-ui-bg/80 text-xs disabled:cursor-default disabled:opacity-40"
        :title="t('components.TrayMusicControl.next')" :aria-label="t('components.TrayMusicControl.next')">
        <ThemeIcon :name="nextIcon" type="symbol" :size="16" :alt="t('components.TrayMusicControl.next')" />
      </button>
    </div>
  </div>
</template>
