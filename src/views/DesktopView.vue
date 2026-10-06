
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import WidgetLayer from '@/components/widgets/WidgetLayer.vue';
import { getBatteryInfo } from '@/services/core.service';
import { wallpaperAssetUrl } from '@/services/wallpaper.service';
import { createWallpaperFollower } from '@/services/wallpaper-colors.service';
import { useSharedEvent } from '@/tools/event.bus';
import { logError, logInfo, logWarning } from '@/utils/logger';
import { fetchVideoBlob, VideoTooLargeError } from '@/utils/video-blob';

const route = useRoute();
const { t } = useI18n();

/**
 * Secondary monitors get a lightweight view: wallpaper only, no widgets or file grid.
 * The backend passes ?monitor=desktop_N for secondary monitors.
 */
const isSecondaryMonitor = computed(() => {
	const monitorParam = route.query.monitor as string | undefined;
	return !!monitorParam && monitorParam !== 'desktop' && monitorParam.startsWith('desktop_');
});

const configStore = useConfigStore();

// Computados reactivos que leen directamente de la configuración del store
const DEFAULT_WALLPAPER = '/usr/share/backgrounds/cutefishos/wallpaper-9.jpg';

const backgroundPath = computed(() => {
	return (configStore as any).config?.desktop?.wallpaper?.[0] || DEFAULT_WALLPAPER;
});

/**
 * Fondos en movimiento.
 *
 * Se reproducen desde un blob, no desde el protocolo de assets: el porqué y el
 * tope de tamaño están en `utils/video-blob.ts`, que comparte con la
 * previsualización del selector de fondos.
 */
const VIDEO_EXTENSIONS = ['mp4', 'webm', 'ogv'] as const;

/**
 * Qué le preguntamos a WebKit para saber si puede con el archivo.
 *
 * Preguntar `video/mp4` a secas no sirve: contesta «maybe» aun cuando no hay
 * ningún decodificador instalado. Con el codec en la pregunta contesta vacío, y
 * ahí sí se puede decidir sin intentar. Un mp4 puede traer H.264 o AV1, así que
 * alcanza con que alguno de los dos sea reproducible.
 */
const CODEC_PROBES: Record<string, string[]> = {
	mp4: ['video/mp4; codecs="avc1.42E01E"', 'video/mp4; codecs="av01.0.04M.08"'],
	webm: ['video/webm; codecs="vp9"', 'video/webm; codecs="vp8"'],
	ogv: ['video/ogg; codecs="theora"'],
};

const backgroundExtension = computed(
	() => backgroundPath.value.toLowerCase().split('.').pop() ?? ''
);

const backgroundIsVideo = computed(() =>
	(VIDEO_EXTENSIONS as readonly string[]).includes(backgroundExtension.value)
);

/**
 * Lo que se dibuja de fondo, en capas.
 *
 * Al cambiar el fondo (desde el selector rápido o desde Configuración) el
 * nuevo **entra con un fundido sobre el anterior**, no con un corte: se suma
 * una capa encima, transparente, que pasa a opaca; cuando terminó, las de abajo
 * se sacan y sus blobs se sueltan. Casi siempre hay una sola capa.
 */
interface BackgroundLayer {
	id: number;
	kind: 'image' | 'video';
	src: string;
	/** Si `src` es un blob nuestro, que hay que soltar al sacar la capa. */
	ownsBlob: boolean;
	visible: boolean;
}

/** Lo que dura el fundido; la guardia del diseño sólo acepta los pasos de la escala. */
const FADE_MS = 300;

const layers = ref<BackgroundLayer[]>([]);
let nextLayerId = 0;
const videoElement = ref<HTMLVideoElement | null>(null);

/** Los `<video>` de cada capa, para poder congelar los de abajo. */
const videoElements = new Map<number, HTMLVideoElement>();

function dropLayer(layer: BackgroundLayer): void {
	videoElements.delete(layer.id);
	if (layer.ownsBlob) URL.revokeObjectURL(layer.src);
}

/** Suma una capa encima y, cuando terminó de entrar, saca las de abajo. */
function showLayer(kind: BackgroundLayer['kind'], src: string, ownsBlob = false): void {
	const first = layers.value.length === 0;
	// El de abajo se congela en su último cuadro mientras el nuevo entra: así
	// nunca hay dos videos decodificándose, ni durante el fundido.
	for (const element of videoElements.values()) element.pause();
	const layer: BackgroundLayer = { id: nextLayerId++, kind, src, ownsBlob, visible: first };
	layers.value = [...layers.value, layer];
	if (first) return;

	requestAnimationFrame(() => {
		layers.value = layers.value.map((item) =>
			item.id === layer.id ? { ...item, visible: true } : item
		);
	});
	setTimeout(() => {
		const index = layers.value.findIndex((item) => item.id === layer.id);
		if (index <= 0) return;
		layers.value.slice(0, index).forEach(dropLayer);
		layers.value = layers.value.slice(index);
	}, FADE_MS + 50);
}

function releaseLayers(): void {
	layers.value.forEach(dropLayer);
	layers.value = [];
}

/** La capa de arriba, que es la que manda sobre el video. */
const topLayer = computed(() => layers.value.at(-1) ?? null);

// Si arriba quedó una imagen, no hay video que reanudar: el de abajo se va con
// el fundido y no tiene que volver a arrancar si vuelve el cable.
watch(topLayer, (layer) => {
	videoElement.value = layer?.kind === 'video' ? (videoElements.get(layer.id) ?? null) : null;
});

function setVideoElement(layer: BackgroundLayer, element: unknown): void {
	const video = (element as HTMLVideoElement | null) ?? null;
	if (video) videoElements.set(layer.id, video);
	else videoElements.delete(layer.id);
	if (layer.id === topLayer.value?.id) videoElement.value = video;
}

/**
 * Un fondo en movimiento cuesta plata en batería.
 *
 * Medido en esta máquina, 1080p30: el escritorio pasa de 4 % a 20 % de un
 * núcleo, y eso con decodificación por hardware —el costo no es decodificar,
 * es que cada cuadro cruza el compositor de WebKit y después el del sistema—.
 * Así que el video se pausa cuando no aporta nada:
 *
 *  · con batería, si la persona lo eligió (por omisión sí);
 *  · cuando la sesión está inactiva o bloqueada, que lo avisa el temporizador
 *    de inactividad por D-Bus, porque desde acá adentro no hay forma de saberlo;
 *  · cuando la página deja de ser visible.
 */
const onBattery = ref(false);
const pausedFromOutside = ref(false);
const pageHidden = ref(false);

const pauseOnBattery = computed(
	() => (configStore as any).config?.desktop?.pausevideoonbattery ?? true
);

const shouldPlay = computed(
	() => !pageHidden.value && !pausedFromOutside.value && !(onBattery.value && pauseOnBattery.value)
);

/** Ya avisamos en esta sesión: el aviso es útil una vez, no cada vez. */
let warnedAboutPower = false;

function canDecode(extension: string): boolean {
	const probe = document.createElement('video');
	const tipos = CODEC_PROBES[extension] ?? [`video/${extension}`];
	return tipos.some((tipo) => probe.canPlayType(tipo) !== '');
}

/** El fondo fijo de siempre, para cuando el video no se puede usar. */
function showDefaultImage(): void {
	showLayer('image', convertFileSrc(DEFAULT_WALLPAPER));
}

async function loadBackground() {
	if (!backgroundIsVideo.value) {
		showLayer('image', await wallpaperAssetUrl(backgroundPath.value));
		return;
	}

	if (!canDecode(backgroundExtension.value)) {
		logError(
			`El fondo ${backgroundPath.value} no se puede reproducir: falta el decodificador ` +
				`para ${backgroundExtension.value}. Se muestra el fondo por omisión. ` +
				'En VasakOS lo instala gst-libav.'
		);
		showDefaultImage();
		return;
	}

	const requested = backgroundPath.value;
	try {
		const url = await fetchVideoBlob(await wallpaperAssetUrl(requested));
		// Mientras se leía, se eligió otro fondo: éste ya no va.
		if (requested !== backgroundPath.value) {
			URL.revokeObjectURL(url);
			return;
		}
		showLayer('video', url, true);
		void warnAboutPowerUse();
	} catch (error) {
		const reason =
			error instanceof VideoTooLargeError
				? `${error.message}: se reproduce desde memoria, así que un archivo así dejaría al ` +
					'escritorio ocupando esa RAM todo el tiempo'
				: String(error);
		logError(`No se pudo leer el fondo ${backgroundPath.value}: ${reason}`);
		showDefaultImage();
	}
}

/**
 * Lleva el elemento al estado que corresponde.
 *
 * Pausar un `<video>` no sólo detiene la imagen: detiene el pipeline de
 * GStreamer detrás, que es donde está el gasto.
 */
function applyPlaybackState() {
	const el = videoElement.value;
	if (!el) return;

	if (shouldPlay.value) {
		el.play().catch((error) => logError(`No se pudo reanudar el fondo: ${error}`));
	} else {
		el.pause();
	}
}

watch([shouldPlay, videoElement], applyPlaybackState);

/**
 * Avisa, una vez, que el fondo en movimiento consume más.
 *
 * Se manda cuando está pasando de verdad —no al configurarlo— porque es ahí
 * cuando la información sirve: si la máquina se calienta o la batería baja
 * rápido, esto explica por qué y dice dónde apagarlo. Sólo desde el monitor
 * principal: con tres pantallas, tres avisos idénticos son ruido.
 */
async function warnAboutPowerUse() {
	if (warnedAboutPower || isSecondaryMonitor.value) return;
	warnedAboutPower = true;

	try {
		await invoke('send_notify', {
			summary: t('views.desktop.videoWallpaperPowerTitle'),
			body: t('views.desktop.videoWallpaperPowerBody'),
			urgency: 'low',
		});
	} catch (error) {
		logError(`No se pudo avisar del consumo del fondo en movimiento: ${error}`);
	}
}

/** El video no arrancó igual: se cae al fondo fijo en vez de dejar la pantalla negra. */
function onVideoError() {
	logError(
		`El fondo ${backgroundPath.value} no se pudo reproducir. Se muestra el fondo por omisión.`
	);
	showDefaultImage();
}

watch(backgroundPath, loadBackground, { immediate: true });
onUnmounted(releaseLayers);

/**
 * Las tres señales que deciden si vale la pena seguir decodificando.
 *
 * La batería y la inactividad no se pueden averiguar desde el webview: la
 * primera la informa el applet, y la segunda la avisa por D-Bus el temporizador
 * de inactividad, que es quien sabe cuándo la pantalla se bloqueó.
 */
const playbackListeners: Array<() => void> = [];

onMounted(async () => {
	playbackListeners.push(
		await listen<{ state?: string }>('battery-update', (event) => {
			onBattery.value = event.payload?.state === 'Discharging';
		})
	);

	playbackListeners.push(
		await listen<boolean>('wallpaper-playback', (event) => {
			pausedFromOutside.value = event.payload === false;
		})
	);

	const onVisibility = () => {
		pageHidden.value = document.hidden;
	};
	document.addEventListener('visibilitychange', onVisibility);
	playbackListeners.push(() => document.removeEventListener('visibilitychange', onVisibility));

	// El estado inicial, porque el applet avisa cuando cambia y puede tardar.
	try {
		const info = await getBatteryInfo<{ state?: string }>();
		onBattery.value = info?.state === 'Discharging';
	} catch {
		// Sin batería —una máquina de escritorio— no hay nada que pausar.
	}
});

onUnmounted(() => {
	playbackListeners.forEach((off) => {
		off();
	});
});

const showHiddenFiles = computed(
	() => (configStore as any).config?.desktop?.showhiddenfiles ?? false
);

watch(showHiddenFiles, () => {
	if (!isSecondaryMonitor.value) {
	}
});

/**
 * «Seguir al fondo»: sólo desde el fondo del monitor principal, para que haya
 * un solo seguidor en todo el escritorio. Ver `wallpaper-colors.service.ts`.
 */
const wallpaperFollower = createWallpaperFollower(undefined, (outcome) => {
	if (outcome === 'applied') logInfo('[wallpaper-colors] colores sacados del fondo nuevo');
	else if (outcome === 'unreadable' || outcome === 'save-failed')
		logWarning(`[wallpaper-colors] no se cambiaron los colores: ${outcome}`);
});
const followWallpaper = () => {
	if (isSecondaryMonitor.value) return;
	wallpaperFollower.sync().catch((error) => {
		logError('[wallpaper-colors] error al seguir el fondo', { error: String(error) });
	});
};

let isMounted = false;

onMounted(async () => {
	isMounted = true;
	await (configStore as any).loadConfig();
	if (!isMounted) return;
	// El fondo pudo cambiar mientras el escritorio no estaba (otra sesión, un
	// arranque): se pone al día al abrir.
	followWallpaper();

	// El escritorio secundario sólo necesita el fondo: ni widgets ni escuchas.
	//
	// El aviso de cambio de tema de los iconos ya no se escucha acá: servía para
	// redibujar los iconos de los archivos del escritorio, que ahora son un
	// widget y se releen solos.
});

onUnmounted(() => {
	isMounted = false;
});

useSharedEvent('config-changed', async () => {
	await (configStore as any).loadConfig();
	followWallpaper();
});
</script>

<template>
  <!-- will-change lo deja en su propia capa de composición: sin eso, cada
       cuadro obliga a WebKit a repintar la página entera, con los iconos y los
       widgets adentro. Y sin la maquinaria de PiP ni de reproducción remota,
       que en un fondo de escritorio no significan nada. -->
  <!-- Las capas del fondo: casi siempre una; dos mientras el nuevo entra con
       el fundido sobre el anterior. -->
  <template v-for="layer in layers" :key="layer.id">
    <video v-if="layer.kind === 'video'" :ref="(element) => setVideoElement(layer, element)" :src="layer.src"
      style="will-change: transform"
      class="w-screen h-screen object-cover absolute z-10 transition-opacity duration-300 ease-ui motion-reduce:transition-none"
      :class="layer.visible ? 'opacity-100' : 'opacity-0'"
      loop autoplay muted playsinline
      preload="auto" disablePictureInPicture disableRemotePlayback
      data-background-layer
      @error="onVideoError"></video>
    <img v-else :src="layer.src" :alt="t('views.desktop.backgroundAlt')"
      class="w-screen h-screen object-cover absolute z-10 transition-opacity duration-300 ease-ui motion-reduce:transition-none"
      :class="layer.visible ? 'opacity-100' : 'opacity-0'"
      data-background-layer />
  </template>

  <!-- Widgets: ahora viven en una cuadrícula con su posición guardada, y se
       mueven, se agregan y se sacan desde el modo edición. Antes estaban
       apilados en un flex centrado, sin posición ni nada que se pudiera tocar. -->
  <WidgetLayer v-if="!isSecondaryMonitor" :config="(configStore as any).config" />
</template>
