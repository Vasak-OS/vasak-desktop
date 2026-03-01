
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getIconSource, getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, nextTick, onMounted, type Ref, ref, watch } from 'vue';
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

const imgSrc: Ref<string> = ref('');
const prevIcon: Ref<string> = ref('');
const nextIcon: Ref<string> = ref('');
const playIcon: Ref<string> = ref('');
const pauseIcon: Ref<string> = ref('');

const commandError = ref('');
const showError = ref(false);
let errorTimeout: ReturnType<typeof setTimeout> | null = null;

const dbusStatus = ref('connected');
const dbusMessage = ref('');

const isPlaying = computed(() => String(musicInfo.value?.status || '').toLowerCase() === 'playing');

async function sendCommand(cmd: string): Promise<void> {
	const player = musicInfo.value?.player || '';
	if (!player) {
		showErrorMessage('No hay reproductor activo');
		return;
	}
	try {
		await invoke(cmd, { player });
		if (showError.value) {
			showError.value = false;
			commandError.value = '';
		}
	} catch (e: any) {
		logError(`[music] Error en comando ${cmd}:`, e);
		const msg = e?.message || e?.toString() || 'Error al ejecutar comando';
		showErrorMessage(msg);
	}
}

function showErrorMessage(msg: string): void {
	commandError.value = msg;
	showError.value = true;
	if (errorTimeout) clearTimeout(errorTimeout);
	errorTimeout = globalThis.setTimeout(() => {
		showError.value = false;
		commandError.value = '';
	}, 3000);
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

onMounted(async () => {
	imgSrc.value = await getIconSource('applications-multimedia');
	prevIcon.value = await getSymbolSource('media-seek-backward');
	nextIcon.value = await getSymbolSource('media-skip-forward');
	playIcon.value = await getSymbolSource('media-playback-start');
	pauseIcon.value = await getSymbolSource('media-playback-pause');
	musicInfo.value = await musicNowPlaying();
	listen('music-playing-update', (event) => {
		const payload = (event.payload || {}) as Partial<MusicInfo>;
		console.log('[MusicWidget] Received update:', payload);
		Object.assign(musicInfo.value, payload);
	});

	listen('dbus-status', (event: any) => {
		const payload = event.payload;
		if (payload.service === 'music') {
			dbusStatus.value = payload.status;
			if (payload.status === 'reconnecting') {
				dbusMessage.value = `Reconectando (intento ${payload.attempt})...`;
			} else if (payload.status === 'failed') {
				dbusMessage.value = payload.message || 'Error de conexión';
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
const TITLE_MAX_PX = 150;

function updateTitleOverflow(): void {
	const container = titleContainer.value;
	const inner = titleInner.value;
	if (!container || !inner) {
		titleOverflow.value = false;
		return;
	}
	container.style.width = `${TITLE_MAX_PX}px`;
	const cw = container.clientWidth;
	const iw = inner.scrollWidth;
	if (iw > cw + 2) {
		titleOverflow.value = true;
		marqueeDistance.value = iw - cw;
		marqueeDuration.value = Math.min(20, Math.max(4, marqueeDistance.value / 30));
	} else {
		titleOverflow.value = false;
		marqueeDistance.value = 0;
		marqueeDuration.value = 0;
	}
}

watch(
	() => musicInfo.value?.title,
	async () => {
		await nextTick();
		updateTitleOverflow();
	}
);
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
	try {
		const defaultIcon = await getIconSource('applications-multimedia');
		if (defaultIcon) {
			imgSrc.value = defaultIcon;
		}
	} catch (e) {
		console.warn('Failed to load default icon:', e);
	}
}
</script>

<template>
  <div
    class="p-4 rounded-corner background flex mb-4 ring-2 ring-primary dark:ring-primary-dark items-center"
  >
    <img
      :src="imgSrc"
      :alt="musicInfo.title"
      :title="musicInfo.title"
      class="w-24 h-24 shrink-0"
      :class="{ 'animate-pulse': isPlaying }"
      @error="onImgError"
    />

    <div class="ml-4 flex flex-col justify-center min-w-0">
      <div class="mb-2 min-w-0">
        <div
          ref="titleContainer"
          class="overflow-hidden"
          :title="musicInfo.title || ''"
        >
          <span
            ref="titleInner"
            class="text-sm font-medium inline-block whitespace-nowrap"
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
            {{ musicInfo.title || "Unknown" }}
          </span>
        </div>
        <div
          class="text-xs text-muted truncate"
          :title="musicInfo.artist || ''"
        >
          {{ musicInfo.artist || "" }}
        </div>
      </div>

      <div
        class="flex items-center pr-1 space-x-1 transition-all duration-150"
        aria-hidden="false"
      >
        <button
          @click.prevent="onPrev"
          class="w-12 h-12 flex items-center justify-center rounded-corner background text-xs"
          title="Anterior"
        >
          <img :src="prevIcon" alt="Anterior" class="w-4 h-4" />
        </button>

        <button
          @click.prevent="onPlayPause"
          class="w-12 h-12 flex items-center justify-center rounded-corner background text-xs"
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
          class="w-12 h-12 flex items-center justify-center rounded-corner background text-xs"
          title="Siguiente"
        >
          <img :src="nextIcon" alt="Siguiente" class="w-4 h-4" />
        </button>
      </div>

      <transition enter-active-class="transition-all duration-300 ease-out" leave-active-class="transition-all duration-300 ease-out" enter-from-class="opacity-0 -translate-y-1" leave-to-class="opacity-0 translate-y-1">
        <div
          v-if="showError"
          class="mt-2 text-xs bg-status-error dark:bg-status-error-dark px-2 py-1 rounded-corner"
        >
          {{ commandError }}
        </div>
      </transition>

      <transition enter-active-class="transition-all duration-300 ease-out" leave-active-class="transition-all duration-300 ease-out" enter-from-class="opacity-0 -translate-y-1" leave-to-class="opacity-0 translate-y-1">
        <div
          v-if="dbusStatus === 'reconnecting' || dbusStatus === 'failed'"
          class="mt-2 text-xs px-2 py-1 rounded"
          :class="[
            dbusStatus === 'reconnecting'
              ? 'text-yellow-600 dark:text-yellow-400 bg-yellow-50 dark:bg-yellow-900/20'
              : 'text-red-500 dark:text-red-400 bg-red-50 dark:bg-red-900/20',
          ]"
        >
          {{ dbusMessage }}
        </div>
      </transition>
    </div>
  </div>
</template>

