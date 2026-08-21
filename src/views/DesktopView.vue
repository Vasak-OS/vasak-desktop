
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { homeDir } from '@tauri-apps/api/path';
import { Command } from '@tauri-apps/plugin-shell';
import { useConfigStore, type VSKConfig } from '@vasakgroup/plugin-config-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import type { Store } from 'pinia';
import { type ComputedRef, computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import WidgetLayer from '@/components/widgets/WidgetLayer.vue';
import type { FileEntry } from '@/interfaces/file';
import { getBatteryInfo } from '@/services/core.service';
import { useSharedEvent } from '@/tools/event.bus';
import { getUserDirectories, loadDirectory } from '@/tools/file.controller';
import { logError } from '@/utils/logger';

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

const configStore = useConfigStore() as Store<
	'config',
	{ config: VSKConfig; loadConfig: () => Promise<void> }
>;
const desktopFiles = ref<FileEntry[]>([]);

// Computados reactivos que leen directamente de la configuración del store
const DEFAULT_WALLPAPER = '/usr/share/backgrounds/cutefishos/wallpaper-9.jpg';

const backgroundPath = computed(() => {
	return (configStore as any).config?.desktop?.wallpaper?.[0] || DEFAULT_WALLPAPER;
});

const background = computed(() => convertFileSrc(backgroundPath.value));

/**
 * Fondos en movimiento.
 *
 * Un `<video src>` apuntando al protocolo de assets de Tauri no funciona, y no
 * por los codecs: el elemento multimedia de WebKit no se sirve del cargador de
 * recursos de la página sino de GStreamer, que no sabe leer de un esquema
 * propio. El handler recibe el pedido, entrega los bytes, y el video igual
 * termina en error 4 (SRC_NOT_SUPPORTED). Con `file://` pasa lo mismo, porque
 * la página no es de ese origen.
 *
 * Lo que sí funciona es traer los bytes nosotros y reproducirlos desde memoria:
 * `fetch` sobre el mismo protocolo —que para datos funciona bien— y un blob.
 * El costo es tener el archivo en memoria, así que hay un límite de tamaño: un
 * fondo en bucle son unas decenas de megas, y si alguien apunta a una película
 * es mejor decírselo que quedarse sin RAM.
 */
const VIDEO_EXTENSIONS = ['mp4', 'webm', 'ogv'] as const;
const MAX_VIDEO_BYTES = 128 * 1024 * 1024;

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

const videoUrl = ref<string | null>(null);
const videoElement = ref<HTMLVideoElement | null>(null);

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

/** El fondo fijo: el configurado, o el de siempre si el video no se puede usar. */
const imageBackground = computed(() =>
	backgroundIsVideo.value ? convertFileSrc(DEFAULT_WALLPAPER) : background.value
);

function canDecode(extension: string): boolean {
	const probe = document.createElement('video');
	const tipos = CODEC_PROBES[extension] ?? [`video/${extension}`];
	return tipos.some((tipo) => probe.canPlayType(tipo) !== '');
}

function releaseVideo() {
	if (videoUrl.value) {
		URL.revokeObjectURL(videoUrl.value);
		videoUrl.value = null;
	}
}

async function loadVideoBackground() {
	releaseVideo();

	if (!backgroundIsVideo.value) return;

	if (!canDecode(backgroundExtension.value)) {
		logError(
			`El fondo ${backgroundPath.value} no se puede reproducir: falta el decodificador ` +
				`para ${backgroundExtension.value}. Se muestra el fondo por omisión. ` +
				'En VasakOS lo instala gst-libav.'
		);
		return;
	}

	try {
		const respuesta = await fetch(background.value);

		if (!respuesta.ok) throw new Error(`respuesta ${respuesta.status}`);

		const largo = Number(respuesta.headers.get('content-length') ?? 0);

		if (largo > MAX_VIDEO_BYTES) {
			logError(
				`El fondo ${backgroundPath.value} pesa ${Math.round(largo / 1024 / 1024)} MB y el ` +
					`límite es ${MAX_VIDEO_BYTES / 1024 / 1024} MB: se reproduce desde memoria, así que ` +
					'un archivo así dejaría al escritorio ocupando esa RAM todo el tiempo.'
			);
			return;
		}

		const bytes = await respuesta.blob();

		if (bytes.size > MAX_VIDEO_BYTES) {
			logError(`El fondo ${backgroundPath.value} superó el límite de tamaño al descargarlo.`);
			return;
		}

		videoUrl.value = URL.createObjectURL(bytes);
		void warnAboutPowerUse();
	} catch (error) {
		logError(`No se pudo leer el fondo ${backgroundPath.value}: ${error}`);
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

watch([shouldPlay, videoUrl], applyPlaybackState);

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
		`El fondo ${backgroundPath.value} no se pudo reproducir. ` +
			'Se muestra el fondo por omisión.'
	);
	releaseVideo();
}

watch(backgroundPath, loadVideoBackground, { immediate: true });
onUnmounted(releaseVideo);

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
	playbackListeners.forEach((off) => off());
});

const showFiles = computed(() => (configStore as any).config?.desktop?.showfiles ?? false);
const showHiddenFiles = computed(
	() => (configStore as any).config?.desktop?.showhiddenfiles ?? false
);
const iconSize: ComputedRef<number> = computed(
	(): number => (configStore as any).config?.desktop?.iconsize ?? 64
);

// Cargar archivos del escritorio
const loadDesktopFiles = async () => {
	if (!showFiles.value) {
		desktopFiles.value = [];
		return;
	}

	try {
		const home = await homeDir();
		const userDirs = await getUserDirectories(home);

		// Buscar el directorio Desktop en las carpetas XDG
		const desktopDir = userDirs.find((dir) => dir.xdgKey === 'XDG_DESKTOP_DIR');

		if (desktopDir) {
			desktopFiles.value = await loadDirectory(desktopDir.path, showHiddenFiles.value);
		} else {
			// Fallback al directorio Desktop tradicional si no se encuentra en XDG
			const desktopPath = `${home}/Desktop`;
			desktopFiles.value = await loadDirectory(desktopPath, showHiddenFiles.value);
		}
	} catch (error) {
		logError('Error loading desktop files:', error);
		desktopFiles.value = [];
	}
};

// Manejar clicks en archivos y carpetas
const handleFileClick = async (file: FileEntry) => {
	if (file.isDirectory) {
		// Abrir el file manager externo en la carpeta seleccionada
		try {
			const cmd = Command.create('vasak-file-manager', [file.path]);
			await cmd.spawn();
		} catch (error) {
			logError('Error al abrir file manager:', error);
		}
	} else {
		// Abrir el archivo con la aplicación predeterminada del sistema
		try {
			const cmd = Command.create('open', [file.path]);
			await cmd.spawn();
		} catch (error) {
			logError('Error al abrir archivo:', file.path);
		}
	}
};

// Ver cambios en showFiles para recargar archivos
watch(showFiles, () => {
	if (!isSecondaryMonitor.value) {
		loadDesktopFiles();
	}
});

watch(showHiddenFiles, () => {
	if (!isSecondaryMonitor.value) {
		loadDesktopFiles();
	}
});

let unlistenTheme: (() => void) | null = null;
let isMounted = false;

onMounted(async () => {
	isMounted = true;
	await (configStore as any).loadConfig();
	if (!isMounted) return;

	// Secondary monitors only need wallpaper — skip file loading and theme listeners
	if (!isSecondaryMonitor.value) {
		await loadDesktopFiles();
		if (!isMounted) return;

		const unlisten = await listen('vicons:theme-changed', () => {
			loadDesktopFiles();
		});

		if (isMounted) {
			unlistenTheme = unlisten;
		} else {
			unlisten();
		}
	}
});

onUnmounted(() => {
	isMounted = false;
	unlistenTheme?.();
});

useSharedEvent('config-changed', async () => {
	await (configStore as any).loadConfig();
	if (!isSecondaryMonitor.value) {
		await loadDesktopFiles();
	}
});
</script>

<template>
  <!-- will-change lo deja en su propia capa de composición: sin eso, cada
       cuadro obliga a WebKit a repintar la página entera, con los iconos y los
       widgets adentro. Y sin la maquinaria de PiP ni de reproducción remota,
       que en un fondo de escritorio no significan nada. -->
  <video v-if="videoUrl" ref="videoElement" :src="videoUrl"
    style="border-radius: 0px; will-change: transform"
    class="w-screen h-screen object-cover absolute z-10" loop autoplay muted playsinline
    preload="auto" disablepictureinpicture disableremoteplayback
    @error="onVideoError"></video>
  <img v-else :src="imageBackground" :alt="t('views.desktop.backgroundAlt')" class="w-screen h-screen object-cover absolute z-10"
    style="border-radius: 0px" />

  <!-- Grid de archivos del escritorio (primary monitor only) -->
  <div v-if="!isSecondaryMonitor && showFiles && desktopFiles.length > 0" class="absolute z-15 w-full h-full overflow-auto px-4 py-14">
    <div class="grid gap-4 content-start" :style="{
      gridTemplateColumns: `repeat(auto-fill, minmax(${40 + iconSize}px, 1fr))`
    }">
      <div v-for="file in desktopFiles" :key="file.path"
        class="flex flex-col items-center justify-start cursor-pointer hover:bg-white/10 rounded-lg p-2 transition-colors"
        :style="{ width: `${(iconSize as number) + 40}px` }" @dblclick="handleFileClick(file)">
        <img v-if="file.icon" :src="file.icon" :alt="file.name" class="mb-1 shrink-0"
          :style="{ width: `${iconSize}px`, height: `${iconSize}px` }" />
        <span class="text-white text-center text-sm warp-break-words max-w-full px-1 py-0.5 rounded"
          style="text-shadow: 0 1px 3px rgba(0,0,0,0.8), 0 0 8px rgba(0,0,0,0.6);"
          :style="{ fontSize: `${Math.max(12, iconSize / 6)}px` }">
          {{ file.name }}
        </span>
      </div>
    </div>
  </div>

  <!-- Widgets: ahora viven en una cuadrícula con su posición guardada, y se
       mueven, se agregan y se sacan desde el modo edición. Antes estaban
       apilados en un flex centrado, sin posición ni nada que se pudiera tocar. -->
  <WidgetLayer v-if="!isSecondaryMonitor" :config="(configStore as any).config" />
</template>
