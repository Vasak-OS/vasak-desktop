<script lang="ts" setup>
import { onMounted, ref, computed, Ref, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { getIconSource, getSymbolSource } from "@vasakgroup/plugin-vicons";


const musicInfo: Ref<any> = ref({
  title: "",
  artist: "",
  player: "",
  artUrl: "",
  status: "",
});

// Watch for artUrl changes to update imgSrc
watch(
  () => musicInfo.value?.artUrl,
  async (newUrl) => {
    if (!newUrl || newUrl.trim() === "") {
      // URL vacía o undefined: usar icono por defecto
      imgSrc.value = await getIconSource("applications-multimedia");
      return;
    }

    const url = newUrl.trim();

    if (url.startsWith("file://")) {
      // Remover prefijo file:// (puede ser file:// o file:///)
      const path = url.replace(/^file:\/+/, "/");
      imgSrc.value = convertFileSrc(path);
    } else if (url.startsWith("http://") || url.startsWith("https://")) {
      // URL remota HTTP/HTTPS: usar directamente
      imgSrc.value = url;
    } else if (url.startsWith("/")) {
      // Ruta absoluta: convertir con convertFileSrc
      imgSrc.value = convertFileSrc(url);
    } else {
      // Otros formatos o relativos: intentar directamente
      imgSrc.value = url;
    }
  },
  { immediate: true }
);

async function onImgError() {
  imgSrc.value = await getIconSource("applications-multimedia");
}

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

// Nuevo: control de visibilidad con display (v-show) y animación de entrada/salida
const visible = ref(false);
const isHiding = ref(false);
let hideTimer: ReturnType<typeof setTimeout> | null = null;
const ANIM_MS = 180;

function onEnter() {
  if (hideTimer) {
    clearTimeout(hideTimer);
    hideTimer = null;
  }
  isHiding.value = false;
  visible.value = true;
}

function onLeave() {
  // iniciar animación de salida
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
      if (val === undefined) continue;
      // asignar individualmente para mantener la referencia reactiva
      musicInfo.value[key] = val;
    }
  });
});
</script>

<template>
  <!-- contenedor con handlers para controlar la visibilidad -->
  <div
    class="p-1 rounded-vsk hover:bg-vsk-primary/30 flex"
    @mouseenter="onEnter"
    @mouseleave="onLeave"
  >
    <!-- imagen del disco -->
    <img
      :src="imgSrc"
      :alt="musicInfo.title"
      :title="musicInfo.title"
      class="w-6 h-6 rounded-full origin-center"
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
      <!-- anterior -->
      <button
        @click.prevent="onPrev"
        class="w-6 h-6 flex items-center justify-center rounded-vsk background text-xs"
        title="Anterior"
      >
        <img :src="prevIcon" alt="Anterior" class="w-4 h-4" />
      </button>

      <!-- play / pause (toggle) -->
      <button
        @click.prevent="onPlayPause"
        class="w-6 h-6 flex items-center justify-center rounded-vsk background text-xs"
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
        class="w-6 h-6 flex items-center justify-center rounded-vsk background text-xs"
        title="Siguiente"
      >
        <img :src="nextIcon" alt="Siguiente" class="w-4 h-4" />
      </button>
    </div>
  </div>
</template>

<style scoped>
/* Animaciones de entrada/salida: fade + slight scale/translate */
@keyframes controlsIn {
  from {
    opacity: 0;
    transform: translateX(6px) scale(0.95);
  }
  to {
    opacity: 1;
    transform: translateX(0) scale(1);
  }
}
@keyframes controlsOut {
  from {
    opacity: 1;
    transform: translateX(0) scale(1);
  }
  to {
    opacity: 0;
    transform: translateX(6px) scale(0.95);
  }
}

.controls-anim-in {
  animation: controlsIn 180ms ease-out forwards;
}

.controls-anim-out {
  animation: controlsOut 160ms ease-in forwards;
}
</style>
