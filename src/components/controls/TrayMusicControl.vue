<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	MarqueeText,
	PanelPill,
	SpinningCover,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { presentWindow, toggleApplet } from '@/services/window.service';
import {
	type ActivityProgress,
	useActivityProgress,
} from '@/tools/composables/useActivityProgress';
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
 * A eso se le suman las tareas en curso de cada aplicación —una copia, una
 * descarga, una exportación, lo que publica por `LauncherEntry`
 * (vasak-desktop#281)—: su icono con su aro y su porcentaje.
 *
 * **Una o varias** (vasak-desktop#295): mientras hay una sola actividad —sólo la
 * música, o sólo una tarea— la píldora es ésa, con su portada o icono, el título
 * que se desliza y la sub-línea. Cuando hay dos o más conviven como una fila de
 * iconos: la música primero y después las tareas por avance, cada una tocable
 * —la música abre el reproductor, la tarea trae su ventana al frente—, cada una un
 * `ActionButton` de sólo icono. La fila no pasa de cuatro ranuras: con más, se
 * muestran tres y un `+N` que dice cuántas quedan. En la fila cada actividad es su
 * icono —el del reproductor o el de la app—; el avance exacto queda en el globo y en
 * la vista de una sola. El toggle del panel es el mismo (`panel.music`): apaga la
 * cápsula entera.
 *
 * **Ancho máximo y título deslizante** (vasak-desktop#261): con una sola actividad
 * la píldora no pasa de lo que miden unos cinco iconos del panel (`max-w-44`), y un
 * título que no entra se desliza (`MarqueeText`) en vez de cortarse. El título
 * entero sigue en el globo.
 */

/** Cuántos iconos entran en la fila antes de agrupar el resto en un `+N`. */
const MAX_ICONS = 4;

/** Una entrada de la fila de actividad: la música o una tarea de una app. */
interface ActivityEntry {
	key: string;
	kind: 'music' | 'task';
	/** El título que se muestra o se lee. */
	title: string;
	/** El avance, de 0 a 100, o `null` si no se conoce (una radio en vivo). */
	progress: number | null;
	/** El nombre de icono del tema, sólo para las tareas. */
	icon?: string;
	/** El id de la ventana, para traerla al frente; sólo para las tareas. */
	windowId?: string;
}

const { t } = useI18n();
const { vertical, hasSurface } = usePanelConfig();
const density = usePanelDensity();

const { musicInfo, imgSrc, position, progress, onImgError, initIcons, initMusicInfo } =
	useMusicPlayer();

const { progressApps } = useActivityProgress();

/**
 * Si algo suena, la música es una actividad más. Un stream que reporta `Playing`
 * sin título igual es música: si no, no aparecería el reproductor que suena.
 */
const musicActive = computed(
	() => Boolean(musicInfo.value.title) || playbackStateOf(musicInfo.value.status) === 'playing'
);

/** El avance de la música para su aro, de 0 a 100, o `null` sin largo conocido. */
const musicRing = computed(() => (musicInfo.value.length > 0 ? progress.value * 100 : null));

/** Todo lo que está pasando ahora: la música primero, después las tareas por avance. */
const activities = computed<ActivityEntry[]>(() => {
	const list: ActivityEntry[] = [];
	if (musicActive.value) {
		list.push({
			key: 'music',
			kind: 'music',
			title: musicInfo.value.title,
			progress: musicRing.value,
			icon: musicInfo.value.desktopEntry || 'applications-multimedia',
		});
	}
	for (const app of progressApps.value) {
		list.push({
			key: `task:${app.id}`,
			kind: 'task',
			title: app.title,
			progress: progressPercent(app.progress) ?? 0,
			icon: app.icon,
			windowId: app.id,
		});
	}
	return list;
});

/** Con dos o más, la píldora pasa a ser la fila de iconos. */
const multi = computed(() => activities.value.length >= 2);

/** La única actividad, cuando hay una; sin ninguna, la música en reposo. */
const topProgress = computed<ActivityProgress | null>(() =>
	activities.value.length === 1 && activities.value[0].kind === 'task'
		? progressApps.value[0]
		: null
);

/** La fila: hasta `MAX_ICONS`; con más, deja lugar para el `+N`. */
const shownActivities = computed(() =>
	activities.value.length > MAX_ICONS ? activities.value.slice(0, MAX_ICONS - 1) : activities.value
);

/** Cuántas actividades quedan fuera de la fila, para el `+N`. */
const overflow = computed(() => activities.value.length - shownActivities.value.length);

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

/** El nombre de la fila para un lector de pantalla: cuántas actividades hay. */
const groupLabel = computed(() =>
	t('components.TrayMusicControl.activityGroup').replace('{0}', String(activities.value.length))
);

/** El `+N`: cuántas actividades quedan sin icono propio. */
const overflowLabel = computed(() =>
	t('components.TrayMusicControl.moreActivities').replace('{0}', String(overflow.value))
);

/** El globo de un icono de la fila: la música con su resumen, o la tarea y su avance. */
const entryTitle = (entry: ActivityEntry) =>
	entry.kind === 'music'
		? musicSummary.value
		: `${entry.title}\n${Math.round(entry.progress ?? 0)} %`;

/** El nombre de un icono de la fila, para quien no lo ve. */
const entryName = (entry: ActivityEntry) =>
	entry.kind === 'music'
		? accessibleName.value
		: `${t('components.TrayMusicControl.goToApp')}: ${entry.title} ${Math.round(entry.progress ?? 0)} %`;

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

/** La fila y la tarea en curso no realzan la píldora entera por el reproductor. */
const expanded = computed(() => (multi.value || showingProgress.value ? false : isOpen.value));

/** Abrir el reproductor anclado. La música manda su propio clic desde la fila. */
async function openPlayer(): Promise<void> {
	try {
		await toggleApplet('music', opener.value);
	} catch (error) {
		logError('[TrayMusicControl] no se pudo abrir el reproductor:', error);
	}
}

/** Traer al frente la ventana de una tarea en curso. */
async function goToWindow(windowId: string): Promise<void> {
	try {
		await presentWindow({ windowId });
	} catch (error) {
		logError('[TrayMusicControl] no se pudo ir a la aplicación:', error);
	}
}

/** El clic de la píldora entera (una sola actividad): el reproductor o la ventana. */
async function onActivate(): Promise<void> {
	if (multi.value) return;
	if (showingProgress.value) {
		if (topProgress.value) await goToWindow(topProgress.value.id);
		return;
	}
	await openPlayer();
}

/** El clic de un icono de la fila: la música abre el reproductor, la tarea su ventana. */
async function onActivityClick(entry: ActivityEntry): Promise<void> {
	if (entry.kind === 'music') return openPlayer();
	if (entry.windowId) await goToWindow(entry.windowId);
}

onMounted(async () => {
	await initIcons();
	await initMusicInfo();
});
</script>

<template>
  <!-- La píldora es un botón entero cuando hay una sola actividad: se apunta
       entera, se realza mientras el reproductor está abierto y es el ancla de
       dónde se despliega. Con dos o más deja de ser botón (no se anidan botones)
       y pasa a ser un grupo con un icono tocable por actividad. -->
  <PanelPill
    ref="opener"
    :interactive="!multi"
    :expanded="expanded"
    :title="multi ? undefined : summary"
    :accessible-label="multi ? groupLabel : accessibleName"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    :flat="hasSurface"
    :flush="multi || showTitle"
    class="max-w-44"
    :class="multi ? 'px-1.5' : showTitle ? 'px-3' : ''"
    :role="multi ? 'group' : undefined"
    data-music-pill
    :data-activity-mode="multi ? 'multi' : showingProgress ? 'progress' : 'music'"
    @click="onActivate"
  >
    <!-- Una sola actividad: la portada que gira (música) o el icono de la
         aplicación que trabaja (progreso). -->
    <template v-if="!multi" #leading>
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
      <ThemeIcon
        v-else
        class="shrink-0"
        :name="topProgress?.icon ?? ''"
        :size="24"
        :alt="currentTitle"
      />
    </template>

    <!-- El título de la única actividad: va en la ranura y no en `label`, para
         que se deslice en vez de cortarse. Con `flush` la píldora no pone relleno
         propio y se lo damos con `px-3`. -->
    <span v-if="!multi && showTitle" class="flex min-w-0 flex-col items-start gap-0.5" data-music-text>
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

    <!-- Dos o más: la fila de iconos. Cada actividad es un `ActionButton` de sólo
         icono —el del reproductor o el de la app—; los botones de la fila son los
         de la librería (no uno a mano) y por eso la píldora deja de serlo. El `+N`
         cierra la fila cuando hay más de las que entran. -->
    <div
      v-if="multi"
      class="flex items-center gap-1"
      :class="vertical ? 'flex-col' : ''"
      data-activity-row
    >
      <ActionButton
        v-for="entry in shownActivities"
        :key="entry.key"
        variant="ghost"
        size="sm"
        label=""
        :icon="entry.icon ?? ''"
        icon-type="icon"
        :icon-alt="entryName(entry)"
        :title="entryTitle(entry)"
        data-activity-icon
        :data-activity-kind="entry.kind"
        @click="onActivityClick(entry)"
      />

      <!-- Las que no entran: un contador, con el globo que dice cuántas. -->
      <span
        v-if="overflow > 0"
        class="grid h-7 min-w-7 place-items-center rounded-corner-full bg-ui-surface/70 px-2 text-label-xs font-medium text-tx-muted"
        :title="overflowLabel"
        :aria-label="overflowLabel"
        data-activity-overflow
      >+{{ overflow }}</span>
    </div>
  </PanelPill>
</template>
