
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getIconSource, getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, type Ref, ref, watch } from 'vue';
import type { MusicInfo } from '@/interfaces/music';
import { musicNowPlaying } from '@/services/audio.service';
import { processImageUrl } from '@/utils/image';
import { logError } from '@/utils/logger';

const musicInfo: Ref<MusicInfo> = ref({
	title: '',
	artist: '',
	player: '',
	artUrl: '',
	status: '',
});

// Watch for artUrl changes to update imgSrc
watch(
	() => musicInfo.value?.artUrl,
	async (newUrl) => {
		const processedUrl = processImageUrl(newUrl);
		if (processedUrl) {
			imgSrc.value = processedUrl;
		} else {
			imgSrc.value = await getIconSource('applications-multimedia');
		}
	},
	{ immediate: true }
);

async function onImgError(): Promise<void> {
	imgSrc.value = await getIconSource('applications-multimedia');
}

const imgSrc: Ref<string> = ref('');
const prevIcon: Ref<string> = ref('');
const nextIcon: Ref<string> = ref('');
const playIcon: Ref<string> = ref('');
const pauseIcon: Ref<string> = ref('');

const isPlaying = computed(() => String(musicInfo.value?.status || '').toLowerCase() === 'playing');

async function sendCommand(cmd: string): Promise<void> {
	const player = musicInfo.value?.player || '';
	if (!player) {
		console.warn('[music] no player bus name available');
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
	imgSrc.value = await getIconSource('applications-multimedia');
	prevIcon.value = await getSymbolSource('media-seek-backward');
	nextIcon.value = await getSymbolSource('media-skip-forward');
	playIcon.value = await getSymbolSource('media-playback-start');
	pauseIcon.value = await getSymbolSource('media-playback-pause');
	musicInfo.value = await musicNowPlaying();
	listen('music-playing-update', (event) => {
		const payload = (event.payload || {}) as Partial<MusicInfo>;
		Object.assign(musicInfo.value, payload);
	});
});
</script>

<template>
  <!-- contenedor con handlers para controlar la visibilidad -->
  <div
    class="p-1 rounded-corner hover:bg-primary hover:dark:bg-primary-dark flex"
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
        class="w-6 h-6 flex items-center justify-center rounded-corner background text-xs"
        title="Anterior"
      >
        <img :src="prevIcon" alt="Anterior" class="w-4 h-4" />
      </button>

      <button
        @click.prevent="onPlayPause"
        class="w-6 h-6 flex items-center justify-center rounded-corner background text-xs"
        :title="isPlaying ? 'Pausa' : 'Reproducir'"
      >
        <img 
          :src="isPlaying ? pauseIcon : playIcon" 
          :alt="isPlaying ? 'Pausa' : 'Reproducir'" 
          class="w-4 h-4" 
        />
      </button>

      <button
        @click.prevent="onNext"
        class="w-6 h-6 flex items-center justify-center rounded-corner background text-xs"
        title="Siguiente"
      >
        <img :src="nextIcon" alt="Siguiente" class="w-4 h-4" />
      </button>
    </div>
  </div>
</template>

