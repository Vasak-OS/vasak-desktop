<script setup lang="ts">
/**
 * El selector rápido de fondos (vasak-desktop#133): una fila de tarjetas
 * inclinadas a media altura, sobre el escritorio y sin contenedor, como en la
 * referencia (§6 de `video-reference.md`).
 *
 * La superficie es una capa a pantalla completa sobre el monitor principal
 * (`windows_apps/wallpaper_picker.rs`). Detrás de la fila sólo hay un velo
 * translúcido —`ui-shell` a la mitad— para que las tarjetas se lean sobre
 * cualquier fondo; sin `backdrop-blur`, que lo pone Wayfire.
 *
 * - Se abre desde el menú del clic derecho del escritorio y por D-Bus
 *   (`OpenWallpaperPicker`, para un atajo de Wayfire).
 * - Enter o un clic aplica (ver `wallpaper.service.ts`) y cierra; Escape o un
 *   clic afuera de la fila, cierra.
 * - Un fondo de video se previsualiza en movimiento al enfocarlo o al pasarle
 *   el puntero, **de a uno**: el carrusel dice cuál (`preview`) y acá se carga
 *   ese solo, soltando el anterior antes (`createPreviewLoader`).
 */
import { convertFileSrc } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { readConfig } from '@vasakgroup/plugin-config-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { AlertMessage, WallpaperCarousel, type WallpaperItem } from '@vasakgroup/vue-libvasak';
import { computed, nextTick, onMounted, onUnmounted, ref, useTemplateRef } from 'vue';
import {
	applyWallpaper,
	currentWallpaper,
	hideWallpaperPicker,
	loadWallpaperCatalog,
	openWallpaperSettings,
	toCarouselItems,
} from '@/services/wallpaper.service';
import { logError } from '@/utils/logger';
import { createPreviewLoader, fetchVideoBlob } from '@/utils/video-blob';

const { t } = useI18n();

const baseItems = ref<WallpaperItem[]>([]);
const current = ref<string | null>(null);
const loading = ref(true);
const applying = ref(false);
const error = ref('');
/** Para la entrada: la fila aparece con un fundido corto cada vez que se abre. */
const shown = ref(false);

const preview = ref<{ id: string | null; url: string | null }>({ id: null, url: null });

const previews = createPreviewLoader({
	load: (path) => fetchVideoBlob(convertFileSrc(path)),
	release: (url) => URL.revokeObjectURL(url),
	onChange: (id, url) => {
		preview.value = { id, url };
	},
});

/** La fila, con la previsualización puesta sólo en el que se está mirando. */
const items = computed<WallpaperItem[]>(() =>
	baseItems.value.map((item) =>
		item.id === preview.value.id && preview.value.url
			? { ...item, videoSrc: preview.value.url }
			: item
	)
);

const carousel = useTemplateRef<{ $el: HTMLElement }>('carousel');

async function load(): Promise<void> {
	loading.value = true;
	error.value = '';
	try {
		current.value = currentWallpaper(await readConfig());
		baseItems.value = toCarouselItems(await loadWallpaperCatalog(current.value));
	} catch (reason) {
		logError(`[wallpaper_picker] no se pudo armar la lista de fondos: ${reason}`);
		baseItems.value = [];
	} finally {
		loading.value = false;
	}
}

/** El foco a la fila, para que las flechas anden sin hacer clic antes. */
async function focusRow(): Promise<void> {
	await nextTick();
	carousel.value?.$el?.focus();
}

async function open(): Promise<void> {
	shown.value = false;
	await load();
	shown.value = true;
	await focusRow();
}

async function close(): Promise<void> {
	previews.clear();
	shown.value = false;
	try {
		await hideWallpaperPicker();
	} catch (reason) {
		logError(`[wallpaper_picker] no se pudo cerrar: ${reason}`);
	}
}

async function onApply(item: WallpaperItem): Promise<void> {
	if (applying.value) return;
	applying.value = true;
	error.value = '';
	try {
		current.value = await applyWallpaper(item.id);
		// El fondo nuevo entra con un fundido en el escritorio (`DesktopView`);
		// el selector se cierra detrás.
		await close();
	} catch (reason) {
		error.value = t('views.wallpaperPicker.applyError').replace('{0}', String(reason));
		logError(`[wallpaper_picker] no se pudo aplicar ${item.id}: ${reason}`);
	} finally {
		applying.value = false;
	}
}

async function onMore(): Promise<void> {
	await close();
	try {
		await openWallpaperSettings();
	} catch (reason) {
		logError(`[wallpaper_picker] no se pudo abrir Configuración: ${reason}`);
	}
}

function onPreview(id: string | null): void {
	void previews.show(id);
}

/**
 * Un clic fuera de la fila cierra, como Escape. La superficie ocupa la pantalla
 * entera, así que ese clic llega acá y no a la ventana de abajo; perder el foco
 * (un clic en otra superficie) ya lo cierra desde GTK.
 */
function onOutsideClick(event: MouseEvent): void {
	const target = event.target as HTMLElement | null;
	if (target?.closest('[data-wallpaper-carousel], [role="alert"]')) return;
	void close();
}

const unlisten: UnlistenFn[] = [];

onMounted(async () => {
	// La superficie se esconde y se vuelve a mostrar, no se destruye: Vue no se
	// vuelve a montar, así que la lista se pide de nuevo al mostrarla. Y al
	// esconderla —Escape, un clic en otra ventana— se suelta el video: una
	// superficie escondida que sigue decodificando es gasto puro.
	const offShown = await listen('window-shown', () => void open());
	const offHidden = await listen('window-hidden', () => {
		previews.clear();
		shown.value = false;
	});
	document.addEventListener('click', onOutsideClick);
	unlisten.push(offShown, offHidden, () => document.removeEventListener('click', onOutsideClick));
	await open();
});

onUnmounted(() => {
	previews.clear();
	for (const off of unlisten.splice(0)) off();
});
</script>

<template>
  <!-- El velo es la raíz. Un clic en él, fuera de la fila, cierra: lo escucha
       `onOutsideClick` en el documento, y el teclado tiene Escape. -->
  <div
    class="flex h-screen w-screen flex-col items-center justify-center bg-ui-shell/50 transition-opacity duration-200 ease-ui-out motion-reduce:transition-none"
    :class="shown ? 'opacity-100' : 'opacity-0'"
    data-wallpaper-picker>
    <!-- Sin margen a los costados: las tarjetas de las puntas se van por el
         borde de la pantalla en vez de cortarse en seco a 16 px de él. -->
    <div class="flex w-full min-w-0 flex-col items-center gap-3">
      <h1 class="sr-only">{{ t('views.wallpaperPicker.title') }}</h1>
      <WallpaperCarousel
        v-if="!loading"
        ref="carousel"
        :items="items"
        :current="current"
        :label="t('views.wallpaperPicker.title')"
        :more-label="t('views.wallpaperPicker.more')"
        :empty-label="t('views.wallpaperPicker.empty')"
        :video-label="t('views.wallpaperPicker.video')"
        :current-label="t('views.wallpaperPicker.current')"
        :aria-busy="applying ? 'true' : undefined"
        @apply="onApply"
        @more="onMore"
        @close="close"
        @preview="onPreview" />
      <p class="px-4 text-center text-label-s text-tx-main text-shadow-legible" data-hint>
        {{ t('views.wallpaperPicker.hint') }}
      </p>
      <AlertMessage v-if="error" tone="error" class="mx-4 max-w-md">{{ error }}</AlertMessage>
    </div>
  </div>
</template>
