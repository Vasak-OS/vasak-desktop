<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { MarqueeText, PanelPill, SpinningCover, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { toggleApplet, toggleWindow } from '@/services/window.service';
import { useActivityProgress } from '@/tools/composables/useActivityProgress';
import { useMusicPlayer } from '@/tools/composables/useMusicPlayer';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { usePanelDensity } from '@/tools/composables/usePanelDensity';
import { showsNames } from '@/tools/panel-density';
import { progressPercent } from '@/tools/tray-item';
import { logError } from '@/utils/logger';
import { formatDuration, playbackStateOf } from '@/utils/playback';

/**
 * La cápsula de actividad del panel: un indicador que muestra lo que está
 * pasando ahora.
 *
 * Nació como el control de música (vasak-desktop#131, #261) y ése sigue siendo su
 * caso principal: la portada que gira con su aro de avance y el título corto, y
 * tocarla despliega el reproductor anclado (`MusicAppletView`, el applet `music`).
 * Cuando no suena nada, la misma píldora pasa a mostrar la aplicación que tiene
 * una tarea en curso —una copia, una descarga, una exportación, lo que publica por
 * `LauncherEntry` (vasak-desktop#281)—: su icono, su título y el porcentaje, con la
 * misma forma. Tocarla ahí trae esa ventana al frente.
 *
 * La música manda: mientras algo suena, eso es lo que se ve. El reproductor y sus
 * comandos siguen viviendo en el applet, no acá. El toggle del panel es el mismo
 * (`panel.music`): apaga la cápsula entera.
 *
 * **Ancho máximo y título deslizante** (vasak-desktop#261): la píldora no pasa de
 * lo que miden unos cinco iconos del panel (`max-w-44`), y un título que no entra
 * se desliza (`MarqueeText`) en vez de cortarse. El título entero sigue en el globo.
 */

const { t } = useI18n();
const { vertical, hasSurface } = usePanelConfig();
const density = usePanelDensity();

const { musicInfo, imgSrc, position, progress, onImgError, initIcons, initMusicInfo } =
	useMusicPlayer();

const { topProgress } = useActivityProgress();

/** Mientras algo suena, la cápsula es el reproductor; si no, la tarea en curso. */
const musicActive = computed(() => Boolean(musicInfo.value.title));
const showingProgress = computed(() => !musicActive.value && topProgress.value !== null);

/** El avance de la tarea en curso, entero de 0 a 100. */
const pct = computed(() => (topProgress.value ? progressPercent(topProgress.value.progress) : 0));

/** El título que se muestra: la pista, o la aplicación que trabaja. */
const currentTitle = computed(() =>
	showingProgress.value ? (topProgress.value?.title ?? '') : musicInfo.value.title
);

/**
 * Lo que dice el globo del reproductor: qué suena, de quién, de qué disco y por
 * dónde va.
 */
const musicSummary = computed(() => {
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

/** El globo de la tarea en curso: qué aplicación, y cuánto lleva. */
const progressSummary = computed(() => {
	const target = topProgress.value;
	if (!target) return '';
	return `${target.title}\n${pct.value} %`;
});

const summary = computed(() =>
	showingProgress.value ? progressSummary.value : musicSummary.value
);

/** El nombre del botón: qué hace y sobre qué, para quien no ve el icono. */
const accessibleName = computed(() => {
	if (showingProgress.value) {
		const target = topProgress.value;
		if (!target) return '';
		return `${t('components.TrayMusicControl.goToApp')}: ${target.title} ${pct.value} %`;
	}
	const open = t('components.TrayMusicControl.openPlayer');
	const info = musicInfo.value;
	if (!info.title) return open;
	return [open, info.artist ? `${info.title} — ${info.artist}` : info.title].join(': ');
});

/** Si suena, está en pausa o no hay nada: el disco gira, se congela o se queda quieto. */
const state = computed(() => playbackStateOf(musicInfo.value.status));

// En un panel angosto queda el icono solo: el título entero sigue en el globo.
const showTitle = computed(
	() => !vertical.value && showsNames(density.value) && Boolean(currentTitle.value)
);

/**
 * La sub-línea: «01:42 / 04:19» para la música —con los minutos en dos cifras—,
 * o «45 %» para la tarea en curso. Sin largo conocido (una radio en vivo), nada.
 */
const clock = (micros: number) => formatDuration(micros).padStart(5, '0');
const timing = computed(() => {
	if (!showTitle.value) return '';
	if (showingProgress.value) return `${pct.value} %`;
	const info = musicInfo.value;
	if (info.length <= 0) return '';
	return `${clock(position.value)} / ${clock(info.length)}`;
});

const opener = ref<unknown>(null);
const { isOpen } = useOpenApplet('music');

/** La tarea en curso no tiene applet: la píldora no queda realzada por ella. */
const expanded = computed(() => (showingProgress.value ? false : isOpen.value));

async function onActivate(): Promise<void> {
	if (showingProgress.value) {
		const target = topProgress.value;
		if (!target) return;
		try {
			await toggleWindow({ windowId: target.id });
		} catch (error) {
			logError('[TrayMusicControl] no se pudo ir a la aplicación:', error);
		}
		return;
	}
	try {
		await toggleApplet('music', opener.value);
	} catch (error) {
		logError('[TrayMusicControl] no se pudo abrir el reproductor:', error);
	}
}

onMounted(async () => {
	await initIcons();
	await initMusicInfo();
});
</script>

<template>
  <!-- La píldora es un botón entero: se apunta entera, se realza mientras el
       reproductor está abierto y es el ancla de dónde se despliega. Adentro, la
       portada que gira (música) o el icono de la aplicación que trabaja
       (progreso), el título —que se desliza si no entra— y la sub-línea. -->
  <PanelPill
    ref="opener"
    :expanded="expanded"
    :title="summary"
    :accessible-label="accessibleName"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    :flat="hasSurface"
    :flush="showTitle"
    class="max-w-44"
    :class="showTitle ? 'px-3' : ''"
    data-music-pill
    :data-activity-mode="showingProgress ? 'progress' : 'music'"
    @click="onActivate"
  >
    <template #leading>
      <SpinningCover
        v-if="!showingProgress"
        class="size-6 shrink-0"
        :src="imgSrc"
        :alt="musicInfo.title"
        :state="state"
        :progress="musicInfo.length > 0 ? progress * 100 : null"
        :progress-label="t('components.TrayMusicControl.progress')"
        @error="onImgError"
      />
      <!-- La tarea en curso: el icono de la aplicación, ya resuelto por el
           backend. El porcentaje va en la sub-línea, como el tiempo de la pista. -->
      <ThemeIcon
        v-else
        class="shrink-0"
        :name="topProgress?.icon ?? ''"
        :size="24"
        :alt="currentTitle"
      />
    </template>
    <!-- El título va en la ranura y no en `label`: el de la píldora se corta con
         puntos suspensivos, y acá tiene que deslizarse. Con `flush` la píldora no
         pone relleno propio y se lo damos con `px-3`. -->
    <span v-if="showTitle" class="flex min-w-0 flex-col items-start gap-0.5" data-music-text>
      <MarqueeText
        class="max-w-full text-label-s leading-none font-medium"
        :text="currentTitle"
        data-music-title
      />
      <span
        v-if="timing"
        class="max-w-full min-w-0 truncate text-label-xs leading-none text-tx-muted"
        data-music-timing
      >{{ timing }}</span>
    </span>
  </PanelPill>
</template>
