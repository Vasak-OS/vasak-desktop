
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { nextTick, onMounted, ref, watch } from 'vue';
import { useMusicPlayer } from '@/tools/composables/useMusicPlayer';
import { logError } from '@/utils/logger';

const {
	musicInfo,
	imgSrc,
	isPlaying,
	prevIcon,
	nextIcon,
	playIcon,
	pauseIcon,
	onPrev,
	onNext,
	onPlayPause,
	onImgError,
	initIcons,
	initMusicInfo,
} = useMusicPlayer();

const commandError = ref('');
const showError = ref(false);
let errorTimeout: ReturnType<typeof setTimeout> | null = null;

const dbusStatus = ref('connected');
const dbusMessage = ref('');

onMounted(async () => {
	await initIcons();
	await initMusicInfo();
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
</script>

<template>
  <div
    class="p-4 rounded-corner bg-ui-bg/80 flex mb-4 ring-2 ring-primary items-center"
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
          class="w-12 h-12 flex items-center justify-center rounded-corner bg-ui-bg/80 text-xs"
          title="Anterior"
        >
          <img :src="prevIcon" alt="Anterior" class="w-4 h-4" />
        </button>

        <button
          @click.prevent="onPlayPause"
          class="w-12 h-12 flex items-center justify-center rounded-corner bg-ui-bg/80 text-xs"
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
          class="w-12 h-12 flex items-center justify-center rounded-corner bg-ui-bg/80 text-xs"
          title="Siguiente"
        >
          <img :src="nextIcon" alt="Siguiente" class="w-4 h-4" />
        </button>
      </div>

      <transition enter-active-class="transition-all duration-300 ease-out" leave-active-class="transition-all duration-300 ease-out" enter-from-class="opacity-0 -translate-y-1" leave-to-class="opacity-0 translate-y-1">
        <div
          v-if="showError"
          class="mt-2 text-xs bg-status-error px-2 py-1 rounded-corner"
        >
          {{ commandError }}
        </div>
      </transition>

      <transition enter-active-class="transition-all duration-300 ease-out" leave-active-class="transition-all duration-300 ease-out" enter-from-class="opacity-0 -translate-y-1" leave-to-class="opacity-0 translate-y-1">
        <div
          v-if="dbusStatus === 'reconnecting' || dbusStatus === 'failed'"
          class="mt-2 text-xs px-2 py-1 rounded-corner  text-ui-main"
          :class="[
            dbusStatus === 'reconnecting'
              ? 'bg-status-warning'
              : 'bg-status-error',
          ]"
        >
          {{ dbusMessage }}
        </div>
      </transition>
    </div>
  </div>
</template>

