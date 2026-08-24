
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { useMusicPlayer } from '@/tools/composables/useMusicPlayer';

const { t } = useI18n();

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
	if (!titleContainer.value) return;

	observadorTitulo = new ResizeObserver(() => updateTitleOverflow());
	observadorTitulo.observe(titleContainer.value);
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
    Dos filas: arriba qué está sonando, abajo los controles ocupando todo el
    ancho. Antes eran tres columnas y los botones quedaban apretados contra el
    borde derecho, con la mitad del ancho desperdiciado entre el título y ellos.

    Las medidas van en unidades de contenedor —cqmin, el lado más chico— así que
    todo acompaña el tamaño de la celda en los dos ejes. El marco (fondo, blur,
    borde) no está acá: lo pone el contenedor de widgets, igual para todos.
  -->
  <div class="relative flex h-full w-full flex-col gap-[2cqmin] p-[3cqmin]">
    <!-- Arriba: la portada y el título. `min-h-0` para que esta fila pueda
         encogerse y los controles nunca queden fuera del widget. -->
    <div class="flex min-h-0 flex-1 items-center gap-[3cqmin]">
      <img
        :src="imgSrc"
        :alt="musicInfo.title"
        :title="musicInfo.title"
        class="aspect-square h-full max-h-full shrink-0 rounded-corner object-cover"
        :class="{ 'animate-pulse': isPlaying }"
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
            {{ musicInfo.title || t('components.MusicWidget.unknownTitle') }}
          </span>
        </div>

        <div
          class="truncate text-[clamp(0.65rem,10cqmin,0.9rem)] text-tx-muted"
          :title="musicInfo.artist || ''"
        >
          {{ musicInfo.artist || "" }}
        </div>
      </div>
    </div>

    <!-- Abajo: los controles, repartidos en partes iguales. Con `flex-1` cada
         botón ocupa lo mismo y la fila llega a los dos bordes, en vez de
         quedar amontonada en una esquina. -->
    <div class="flex shrink-0 items-stretch gap-[2cqmin]">
      <button
        class="flex h-[clamp(1.25rem,20cqmin,2.5rem)] flex-1 items-center justify-center rounded-corner bg-ui-surface/60 transition-colors hover:bg-ui-surface"
        :title="t('components.MusicWidget.previous')"
        @click.prevent="onPrev"
      >
        <img :src="prevIcon" :alt="t('components.MusicWidget.previous')" class="h-[55%] w-auto" />
      </button>

      <button
        class="flex h-[clamp(1.25rem,20cqmin,2.5rem)] flex-[1.4] items-center justify-center rounded-corner bg-primary/80 transition-colors hover:bg-primary"
        :title="isPlaying ? t('components.MusicWidget.pause') : t('components.MusicWidget.play')"
        @click.prevent="onPlayPause"
      >
        <img
          :src="isPlaying ? pauseIcon : playIcon"
          :alt="isPlaying ? t('components.MusicWidget.pause') : t('components.MusicWidget.play')"
          class="h-[60%] w-auto"
        />
      </button>

      <button
        class="flex h-[clamp(1.25rem,20cqmin,2.5rem)] flex-1 items-center justify-center rounded-corner bg-ui-surface/60 transition-colors hover:bg-ui-surface"
        :title="t('components.MusicWidget.next')"
        @click.prevent="onNext"
      >
        <img :src="nextIcon" :alt="t('components.MusicWidget.next')" class="h-[55%] w-auto" />
      </button>
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
        <div v-if="showError" class="rounded-corner bg-status-error px-2 py-1 text-xs">
          {{ commandError }}
        </div>
      </transition>

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

