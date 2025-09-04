<script lang="ts" setup>
import { onMounted, ref, computed, Ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getIconSource, getSymbolSource } from "@vasakgroup/plugin-vicons";

const musicInfo: Ref<any> = ref({
  title: "",
  artist: "",
  player: "",
  artUrl: "",
  status: "",
});

const imgSrc: Ref<string> = ref("");
const prevIcon: Ref<string> = ref("");
const nextIcon: Ref<string> = ref("");
const playIcon: Ref<string> = ref("");
const pauseIcon: Ref<string> = ref("");

// Nuevo: computed que devuelve true si el status (normalizado) es "playing"
const isPlaying = computed(() => {
  const s = musicInfo.value?.status;
  if (!s) return false;
  return String(s).toLowerCase() === "playing";
});

// helpers para invocar comandos Tauri (el frontend siempre pasa el player)
async function sendCommand(cmd: string) {
  const player = musicInfo.value?.player || "";
  if (!player) {
    console.warn("[music] no player bus name available");
    return;
  }
  try {
    await invoke(cmd, { player });
  } catch (e) {
    console.error(`[music] invoke ${cmd} failed:`, e);
  }
}
function onPrev() {
  sendCommand("music_previous_track");
}
function onNext() {
  sendCommand("music_next_track");
}
function onPlayPause() {
  sendCommand("music_play_pause");
}

onMounted(async () => {
  imgSrc.value = await getIconSource("applications-multimedia");
  prevIcon.value = await getSymbolSource("media-seek-backward");
  nextIcon.value = await getSymbolSource("media-skip-forward");
  playIcon.value = await getSymbolSource("media-playback-start");
  pauseIcon.value = await getSymbolSource("media-playback-pause");
  musicInfo.value = await invoke("music_now_playing");
  listen("music-playing-update", (event) => {
    const payload = (event.payload || {}) as Record<string, unknown>;
    for (const key of Object.keys(payload)) {
      const val = payload[key];
      if (val === undefined || val === null) continue;
      if (typeof val === "string") {
        if (val.trim() === "") continue;
      }
      musicInfo.value[key] = val;
    }
  });
});
</script>

<template>
  <div class="p-4 rounded-vsk background flex mb-4 ring-2 ring-vsk-primary/50 items-center">
    <img
      :src="imgSrc"
      :alt="musicInfo.title"
      :title="musicInfo.title"
      class="w-24 h-24 flex-shrink-0"
      :class="{ 'animate-pulse': isPlaying }"
    />

    <div class="ml-4 flex flex-col justify-center min-w-0">
      <!-- Título y artista -->
      <div class="mb-2 min-w-0">
        <div
          class="text-sm font-medium truncate"
          :title="musicInfo.title || ''"
        >
          {{ musicInfo.title || 'Unknown' }}
        </div>
        <div
          class="text-xs text-muted truncate"
          :title="musicInfo.artist || ''"
        >
          {{ musicInfo.artist || '' }}
        </div>
      </div>

      <!-- controles en línea (igual que antes) -->
      <div
        class="flex items-center pr-1 space-x-1 transition-all duration-150"
        aria-hidden="false"
      >
        <!-- anterior -->
        <button
          @click.prevent="onPrev"
          class="w-12 h-12 flex items-center justify-center rounded-vsk background text-xs"
          title="Anterior"
        >
          <img :src="prevIcon" alt="Anterior" class="w-4 h-4" />
        </button>

        <!-- play / pause (toggle) -->
        <button
          @click.prevent="onPlayPause"
          class="w-12 h-12 flex items-center justify-center rounded-vsk background text-xs"
          :title="isPlaying ? 'Pausa' : 'Reproducir'"
        >
          <template v-if="isPlaying">
            <img :src="pauseIcon" alt="Pausa" class="w-4 h-4" />
          </template>
          <template v-else>
            <img :src="playIcon" alt="Reproducir" class="w-4 h-4" />
          </template>
        </button>

        <!-- siguiente -->
        <button
          @click.prevent="onNext"
          class="w-12 h-12 flex items-center justify-center rounded-vsk background text-xs"
          title="Siguiente"
        >
          <img :src="nextIcon" alt="Siguiente" class="w-4 h-4" />
        </button>
      </div>
    </div>
  </div>
</template>
