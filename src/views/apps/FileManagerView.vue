<script setup lang="ts">
import { WindowFrame } from '@vasakgroup/vue-libvasak';
import { ref, onMounted } from 'vue';
import { join, homeDir } from '@tauri-apps/api/path';
import { convertFileSrc } from '@tauri-apps/api/core';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { Command } from '@tauri-apps/plugin-shell';
import { useRoute } from 'vue-router';
import type { FileEntry } from '@/interfaces/file';
import {
	loadDirectoryBackend,
	getFileEmoji,
	getUserDirectories,
} from '@/tools/file.controller';

interface SidebarItem {
  name: string;
  icon: string;
  path: string;
}

// State
const route = useRoute();
const sidebarItems = ref([] as SidebarItem[]);
const files = ref<FileEntry[]>([]);
const loading = ref(false);
const error = ref('');
const currentPath = ref('');
const homePath = ref('');
const showHidden = ref(false);
const sidebarIcons = ref<Record<string, string>>({});

const loadFiles = async (path: string) => {
	loading.value = true;
	error.value = '';
	try {
		files.value = await loadDirectoryBackend(path, showHidden.value);
		currentPath.value = path;
	} catch (err: any) {
		error.value = err.toString();
	} finally {
		loading.value = false;
	}
};

const toggleHiddenFiles = () => {
	showHidden.value = !showHidden.value;
	loadFiles(currentPath.value);
};

const navigateTo = (path: string) => {
	loadFiles(path);
};

const navigateUp = () => {
	const parts = currentPath.value.split('/');
	if (parts.length > 1) {
		parts.pop();
		const newPath = parts.join('/') || '/';
		loadFiles(newPath);
	}
};

const handleItemClick = async (file: FileEntry) => {
	if (file.isDirectory) {
		navigateTo(file.path);
	} else {
		try {
			const cmd = Command.create('open', [file.path]);
			await cmd.spawn();
		} catch (e) {
			console.error('Failed to open file:', file.path, e);
		}
	}
};

const handleSidebarClick = (item: SidebarItem) => {
	if (item.path === 'HOME') {
		navigateTo(homePath.value);
	} else if (item.path.startsWith('/')) {
		navigateTo(item.path);
	} else {
		navigateTo(`${homePath.value}/${item.path}`);
	}
};

const loadSidebar = async () => {
	const items: SidebarItem[] = [];

	items.push({ name: 'Home', icon: 'user-home', path: 'HOME' });

	// Load user directories from XDG config
	const userDirectories = await getUserDirectories(homePath.value);
	// Filter out templates and public share directories
	const filteredDirectories = userDirectories.filter(
		(dir) => dir.xdgKey !== 'XDG_TEMPLATES_DIR' && dir.xdgKey !== 'XDG_PUBLICSHARE_DIR'
	);
	items.push(...filteredDirectories);

	const trashPath = await join(
		homePath.value,
		'.local',
		'share',
		'Trash',
		'files'
	);
	items.push({ name: 'Trash', icon: 'user-trash', path: trashPath });

	sidebarItems.value = items;

	for (const item of sidebarItems.value) {
		try {
			const source = await getIconSource(item.icon);
			sidebarIcons.value[item.name] = source.startsWith('/')
				? convertFileSrc(source)
				: source;
		} catch (e) {
			console.warn('Sidebar icon fail', item.icon);
			console.error('Sidebar icon error: ', e);
		}
	}
};

onMounted(async () => {
	try {
		homePath.value = await homeDir();

		// Verroute = useRoute();
		const initialPath = route.query.path as string || homePath.value;

		currentPath.value = initialPath;

		await loadSidebar();

		loadFiles(initialPath);
	} catch (e) {
		console.error('Failed to get home dir', e);
	}
});
</script>

<template>
  <WindowFrame class="rounded-t-window">
    <div class="h-[calc(100vh-32px)] w-screen rounded-b-window background flex flex-col overflow-hidden">
      <div class="h-12 border-b border-vsk-primary flex items-center px-4 background backdrop-blur-sm shrink-0">
        <div class="flex gap-2 mr-4">
          <button @click="navigateUp" aria-label="up" class="hover:text-vsk-primary transition-colors text-lg">
            ↑
          </button>
        </div>
        <div class="flex-1 background rounded-vsk px-3 py-1 text-sm font-medium opacity-80 backdrop-blur-md truncate">
          {{ currentPath }}
        </div>
        <div class="ml-4 flex gap-2 items-center">
          <label class="flex items-center gap-2 text-xs font-medium cursor-pointer select-none">
            <input type="checkbox" :checked="showHidden" @change="toggleHiddenFiles"
              class="rounded-vsk background focus:ring-vsk-primary text-vsk-primary" />
            Show Hidden
          </label>
          <button aria-label="Search" class="p-1 hover:bg-vsk-primary rounded-vsk">
            🔍
          </button>
        </div>
      </div>

      <div class="flex-1 flex min-h-0">
        <!-- Sidebar -->
        <div class="w-48 p-2 border-r dark:border-vsk-primary flex flex-col gap-1 background">
          <button v-for="item in sidebarItems" :key="item.name" @click="handleSidebarClick(item)"
            class="flex items-center gap-3 px-3 py-2 rounded-vsk hover:bg-white/40 dark:hover:bg-white/10 transition-colors text-sm font-medium text-left">
            <img v-if="sidebarIcons[item.name]" :src="sidebarIcons[item.name]" alt="Icon" class="w-5 h-5" />
            <span v-else>📁</span>
            <span>{{ item.name }}</span>
          </button>
        </div>

        <!-- Main Content (Grid View) -->
        <div class="flex-1 p-4 overflow-y-auto">
          <div v-if="loading" class="flex justify-center items-center h-full text-gray-400">
            Loading...
          </div>
          <div v-else-if="error" class="flex justify-center items-center h-full text-red-400">
            {{ error }}
          </div>
          <div v-else class="grid grid-cols-[repeat(auto-fill,minmax(100px,1fr))] gap-4">
            <div v-for="file in files" :key="file.name" @dblclick="handleItemClick(file)"
              class="group flex flex-col items-center gap-2 p-3 rounded-vsk hover:bg-vsk-primary/80 hover:ring-1 hover:ring-vsk-primary cursor-pointer transition-all">
              <div
                class="w-16 h-16 flex items-center justify-center overflow-hidden rounded-vsk background group-hover:scale-105 transition-transform duration-200">
                <!-- Image Preview -->
                <img v-if="
                  file.mimeType === 'image' &&
                  file.previewUrl &&
                  !file.loadError
                " :src="file.previewUrl" class="w-full h-full object-cover" alt="Preview" loading="lazy"
                  @error="file.loadError = true" />

                <!-- Video Preview (Icon fallback if needed, but video tag for now) -->
                <video v-else-if="
                  file.mimeType === 'video' &&
                  file.previewUrl &&
                  !file.loadError
                " :src="file.previewUrl" class="w-full h-full object-cover" muted loop
                  @mouseenter="(e) => (e.target as HTMLVideoElement).play()"
                  @mouseleave="(e) => (e.target as HTMLVideoElement).pause()" @error="file.loadError = true"></video>

                <img v-else-if="file.icon" :src="file.icon" alt="File Icon"
                  class="w-full h-full object-contain p-2 opacity-80" />

                <!-- Fallback Text Icon -->
                <span v-else class="text-3xl opacity-50">{{
                  getFileEmoji(file.name, file.isDirectory)
                  }}</span>
              </div>

              <span
                class="text-xs text-center truncate w-full px-1 select-none font-medium text-gray-700 dark:text-gray-300 group-hover:text-vsk-primary"
                :title="file.name">
                {{ file.name }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </WindowFrame>
</template>

<style scoped>
/* Ensure custom properties are available if needed, though Tailwind handles most */
.rounded-t-window {
  border-top-left-radius: var(--radius-window);
  border-top-right-radius: var(--radius-window);
}

.rounded-b-window {
  border-bottom-left-radius: var(--radius-window);
  border-bottom-right-radius: var(--radius-window);
}
</style>
