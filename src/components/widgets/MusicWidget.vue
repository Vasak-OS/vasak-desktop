<script lang="ts" setup>
import { onMounted, ref, computed, Ref, nextTick, watch } from "vue";
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

const imgSrc: Ref<string> = ref("");
const prevIcon: Ref<string> = ref("");
const nextIcon: Ref<string> = ref("");
const playIcon: Ref<string> = ref("");
const pauseIcon: Ref<string> = ref("");

// Estado de errores
const commandError = ref("");
const showError = ref(false);
let errorTimeout: number | null = null;

// Estado de conexión D-Bus
const dbusStatus = ref("connected");
const dbusMessage = ref("");

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
    showErrorMessage("No hay reproductor activo");
    return;
  }
  try {
    await invoke(cmd, { player });
    // Limpiar error si existía
    if (showError.value) {
      showError.value = false;
      commandError.value = "";
    }
  } catch (e: any) {
    console.error(`[music] invoke ${cmd} failed:`, e);
    const msg = e?.message || e?.toString() || "Error al ejecutar comando";
    showErrorMessage(msg);
  }
}

function showErrorMessage(msg: string) {
  commandError.value = msg;
  showError.value = true;
  if (errorTimeout) clearTimeout(errorTimeout);
  errorTimeout = window.setTimeout(() => {
    showError.value = false;
    commandError.value = "";
  }, 3000);
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
    console.log("[MusicWidget] Received update:", payload);
    for (const key of Object.keys(payload)) {
      const val = payload[key];
      if (val === undefined) continue;
      musicInfo.value[key] = val;
    }
  });
  
  // Escuchar estado de conexión D-Bus
  listen("dbus-status", (event: any) => {
    const payload = event.payload;
    if (payload.service === "music") {
      dbusStatus.value = payload.status;
      if (payload.status === "reconnecting") {
        dbusMessage.value = `Reconectando (intento ${payload.attempt})...`;
      } else if (payload.status === "failed") {
        dbusMessage.value = payload.message || "Error de conexión";
      } else if (payload.status === "connected") {
        dbusMessage.value = "";
      }
    }
  });
  // medir posible overflow del título inicialmente
  await nextTick();
  updateTitleOverflow();
});

// Refs y estado para la animación del título (marquee)
const titleContainer = ref<HTMLElement | null>(null);
const titleInner = ref<HTMLElement | null>(null);
const titleOverflow = ref(false);
const marqueeDistance = ref(0); // px
const marqueeDuration = ref(6); // s, calculado dinámicamente
const TITLE_MAX_PX = 150;

function updateTitleOverflow() {
  // Esperar al siguiente tick si es necesario (usar cuando se llame desde watch)
  const container = titleContainer.value;
  const inner = titleInner.value;
  if (!container || !inner) {
    titleOverflow.value = false;
    return;
  }
  // Forzar ancho máximo del contenedor a 150px
  container.style.width = `${TITLE_MAX_PX}px`;
  // Medir
  const cw = container.clientWidth;
  const iw = inner.scrollWidth;
  if (iw > cw + 2) {
    titleOverflow.value = true;
    marqueeDistance.value = iw - cw;
    // duración proporcional a distancia, mínimo 4s, máximo 20s
    marqueeDuration.value = Math.min(
      20,
      Math.max(4, marqueeDistance.value / 30)
    );
  } else {
    titleOverflow.value = false;
    marqueeDistance.value = 0;
    marqueeDuration.value = 0;
  }
}

// Volver a medir cuando cambie el título (se ejecuta tras cada actualización de DOM)
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
  try {
    const defaultIcon = await getIconSource("applications-multimedia");
    if (defaultIcon) {
       imgSrc.value = defaultIcon;
    }
  } catch (e) {
    console.warn("Failed to load default icon:", e);
  }
}
</script>

<template>
  <!-- contenedor con imagen + columna de info + controles -->
  <div
    class="p-4 rounded-vsk background flex mb-4 ring-2 ring-vsk-primary/50 items-center"
  >
    <img
      :src="imgSrc"
      :alt="musicInfo.title"
      :title="musicInfo.title"
      class="w-24 h-24 flex-shrink-0"
      :class="{ 'animate-pulse': isPlaying }"
      @error="onImgError"
    />

    <!-- columna con título arriba y controls debajo -->
    <div class="ml-4 flex flex-col justify-center min-w-0">
      <!-- Título y artista -->
      <div class="mb-2 min-w-0">
        <!-- contenedor fijo a 150px y ocultar overflow -->
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

      <!-- Toast de error -->
      <transition name="error-fade">
        <div
          v-if="showError"
          class="mt-2 text-xs text-red-500 dark:text-red-400 bg-red-50 dark:bg-red-900/20 px-2 py-1 rounded"
        >
          {{ commandError }}
        </div>
      </transition>
      
      <!-- Estado de reconexión -->
      <transition name="error-fade">
        <div
          v-if="dbusStatus === 'reconnecting' || dbusStatus === 'failed'"
          class="mt-2 text-xs px-2 py-1 rounded"
          :class="[
            dbusStatus === 'reconnecting' ? 'text-yellow-600 dark:text-yellow-400 bg-yellow-50 dark:bg-yellow-900/20' : 'text-red-500 dark:text-red-400 bg-red-50 dark:bg-red-900/20'
          ]"
        >
          {{ dbusMessage }}
        </div>
      </transition>
    </div>
  </div>
</template>

<style scoped>
/* Transición de fade para el toast de error */
.error-fade-enter-active,
.error-fade-leave-active {
  transition: opacity 0.3s ease, transform 0.3s ease;
}
.error-fade-enter-from {
  opacity: 0;
  transform: translateY(-4px);
}
.error-fade-leave-to {
  opacity: 0;
  transform: translateY(4px);
}

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

/* Marquee: mueve el span horizontalmente según variables CSS:
   --marquee-distance (px) y --marquee-duration (s). */
.marquee {
  display: inline-block;
  /* usar animation alternate para ir y volver */
  animation-name: marquee;
  animation-timing-function: linear;
  animation-iteration-count: infinite;
  animation-direction: alternate;
  animation-duration: var(--marquee-duration, 6s);
}

@keyframes marquee {
  from {
    transform: translateX(0);
  }
  to {
    transform: translateX(calc(var(--marquee-distance, 0px) * -1));
  }
}
</style>
