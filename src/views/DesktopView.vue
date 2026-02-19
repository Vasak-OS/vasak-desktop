<script lang="ts" setup>
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { homeDir } from '@tauri-apps/api/path';
import { Command } from '@tauri-apps/plugin-shell';
import { useConfigStore, type VSKConfig } from '@vasakgroup/plugin-config-manager';
import type { Store } from 'pinia';
import { type ComputedRef, computed, onMounted, onUnmounted, ref, watch } from 'vue';
import type { FileEntry } from '@/interfaces/file';
import { getUserDirectories, loadDirectory } from '@/tools/file.controller';
import { logError } from '@/utils/logger';

const configStore = useConfigStore() as Store<
	'config',
	{ config: VSKConfig; loadConfig: () => Promise<void> }
>;
const desktopFiles = ref<FileEntry[]>([]);

let unlistenConfigChanged: (() => void) | null = null;

// Computados reactivos que leen directamente de la configuración del store
const backgroundPath = computed(() => {
	return (
		(configStore as any).config?.desktop?.wallpaper?.[0] ||
		'/usr/share/backgrounds/cutefishos/wallpaper-9.jpg'
	);
});

const _background = computed(() => convertFileSrc(backgroundPath.value));

const _backgroundType = computed(() => {
	const ext = backgroundPath.value.toLowerCase().split('.').pop();
	if (ext === 'mp4' || ext === 'webm' || ext === 'ogv') {
		return `video/${ext}`;
	}
	return `image/${ext || 'jpeg'}`;
});

const showFiles = computed(() => (configStore as any).config?.desktop?.showfiles ?? false);
const showHiddenFiles = computed(
	() => (configStore as any).config?.desktop?.showhiddenfiles ?? false
);
const _iconSize: ComputedRef<number> = computed(
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
const _handleFileClick = async (file: FileEntry) => {
	if (file.isDirectory) {
		// Abrir el file manager en la carpeta seleccionada
		try {
			await invoke('open_file_manager_window', { path: file.path });
		} catch (error) {
			logError('Error al abrir file manager:', error);
		}
	} else {
		// Abrir el archivo con la aplicación predeterminada del sistema
		try {
			const cmd = Command.create('open', [file.path]);
			await cmd.spawn();
		} catch (error) {
			logError('Error al abrir archivo:', file.path, error);
		}
	}
};

// Ver cambios en showFiles para recargar archivos
watch(showFiles, () => {
	loadDesktopFiles();
});

watch(showHiddenFiles, () => {
	loadDesktopFiles();
});

onMounted(async () => {
	await (configStore as any).loadConfig();
	await loadDesktopFiles();
	unlistenConfigChanged = await listen('config-changed', async () => {
		await (configStore as any).loadConfig();
		await loadDesktopFiles();
	});
});

onUnmounted(() => {
	if (unlistenConfigChanged) {
		unlistenConfigChanged();
	}
});
</script>

<template>
  <video v-if="backgroundType.includes('video')" style="border-radius: 0px" :type="backgroundType" :src="background"
    class="w-screen h-screen object-cover absolute z-10" loop autoplay muted></video>
  <img v-else :src="background" alt="Background" class="w-screen h-screen object-cover absolute z-10"
    style="border-radius: 0px" />

  <!-- Grid de archivos del escritorio -->
  <div v-if="showFiles && desktopFiles.length > 0" class="absolute z-15 w-full h-full overflow-auto px-4 py-14">
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

  <main class="w-screen h-screen flex flex-col items-center justify-center absolute z-20 pointer-events-none">
    <MusicWidget class="pointer-events-auto" />
    <DesktopClockWidget class="pointer-events-auto" />
  </main>
</template>
