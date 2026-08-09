
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { onMounted, ref } from 'vue';
import { useMusicPlayer } from '@/tools/composables/useMusicPlayer';

const { t } = useI18n();

const {
	musicInfo,
	imgSrc,
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
</script>

<template>
  <!-- contenedor con handlers para controlar la visibilidad -->
  <div
    class="p-1 rounded-corner hover:bg-primary flex"
    @mouseenter="onEnter"
    @mouseleave="onLeave"
  >
    <!-- imagen del disco -->
    <img
      :src="imgSrc"
      :alt="musicInfo.title"
      :title="musicInfo.title"
      class="w-5.5 h-5.5 rounded-full origin-center"
      :class="{ 'animate-spin': isPlaying }"
      @error="onImgError"
    />

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
      <button
        @click.prevent="onPrev"
        class="w-6 h-6 flex items-center justify-center rounded-corner bg-ui-bg/80 text-xs"
        :title="t('components.TrayMusicControl.previous')"
      >
        <img :src="prevIcon" :alt="t('components.TrayMusicControl.previous')" class="w-4 h-4" />
      </button>

      <button
        @click.prevent="onPlayPause"
        class="w-6 h-6 flex items-center justify-center rounded-corner bg-ui-bg/80 text-xs"
        :title="isPlaying
          ? t('components.TrayMusicControl.pause')
          : t('components.TrayMusicControl.play')"
      >
        <img
          :src="isPlaying ? pauseIcon : playIcon"
          :alt="isPlaying
            ? t('components.TrayMusicControl.pause')
            : t('components.TrayMusicControl.play')"
          class="w-4 h-4"
        />
      </button>

      <button
        @click.prevent="onNext"
        class="w-6 h-6 flex items-center justify-center rounded-corner bg-ui-bg/80 text-xs"
        :title="t('components.TrayMusicControl.next')"
      >
        <img :src="nextIcon" :alt="t('components.TrayMusicControl.next')" class="w-4 h-4" />
      </button>
    </div>
  </div>
</template>

