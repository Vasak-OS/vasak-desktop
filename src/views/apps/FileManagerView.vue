<script setup lang="ts">
import { WindowFrame } from "@vasakgroup/vue-libvasak";
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { homeDir } from '@tauri-apps/api/path';

// Interfaces
interface FileEntry {
  name: string;
  is_dir: boolean;
  size: string;
  path: string;
}

// State
const sidebarItems = [
  { name: "Home", icon: "🏠", path: "HOME" }, // Special marker for home
  { name: "Documents", icon: "📄", path: "Documents" },
  { name: "Downloads", icon: "⬇️", path: "Downloads" },
  { name: "Pictures", icon: "🖼️", path: "Pictures" },
  { name: "Music", icon: "🎵", path: "Music" },
];

const files = ref<FileEntry[]>([]);
const currentPath = ref("");
const homePath = ref("");
const loading = ref(false);
const error = ref("");

// Actions
const loadFiles = async (path: string) => {
  loading.value = true;
  error.value = "";
  try {
    const entries = await invoke<FileEntry[]>("read_directory", { path });
    files.value = entries;
    currentPath.value = path;
  } catch (err: any) {
    error.value = err.toString();
    console.error("Error reading directory:", err);
  } finally {
    loading.value = false;
  }
};

const navigateTo = (path: string) => {
  loadFiles(path);
};

const navigateUp = () => {
  // Simple string manipulation for now, or use path API if available in frontend
  const parts = currentPath.value.split('/');
  if (parts.length > 1) {
    parts.pop();
    const newPath = parts.join('/') || '/'; // Handle root case
    loadFiles(newPath);
  }
};

const handleItemClick = (file: FileEntry) => {
  if (file.is_dir) {
    navigateTo(file.path);
  } else {
    // TODO: Implement file opening logic
    console.log("File clicked:", file.path);
  }
};

const handleSidebarClick = (item: typeof sidebarItems[0]) => {
  if (item.path === "HOME") {
    navigateTo(homePath.value);
  } else {
    navigateTo(`${homePath.value}/${item.path}`);
  }
};

onMounted(async () => {
    try {
        homePath.value = await homeDir();
        currentPath.value = homePath.value;
        loadFiles(homePath.value);
    } catch (e) {
        console.error("Failed to get home dir", e);
        // Fallback or error state
    }
});

/* 
  Design Note:
  Using --radius-window for the main container and --radius-vsk for inner elements.
  Using --color-vsk-primary for accents.
  Tailwind classes handles layout and spacing.
*/
</script>

<template>
  <WindowFrame class="rounded-t-window">
    <div class="h-[calc(100vh-32px)] w-screen rounded-b-window background text-gray-900 dark:text-gray-100 flex flex-col overflow-hidden">
      
      <!-- Toolbar / Breadcrumb Area -->
      <div class="h-12 border-b border-gray-200 dark:border-gray-700/50 flex items-center px-4 bg-white/30 dark:bg-black/20 backdrop-blur-sm shrink-0">
        <div class="flex gap-2 mr-4 text-gray-500">
           <button @click="navigateUp" class="hover:text-vsk-primary transition-colors text-lg">↑</button>
           <!-- Back/Forward could be implemented with history stack later -->
        </div>
        <div class="flex-1 bg-white/40 dark:bg-black/20 rounded-[var(--radius-vsk)] px-3 py-1 text-sm font-medium opacity-80 backdrop-blur-md truncate">
          {{ currentPath }}
        </div>
        <div class="ml-4 flex gap-2">
            <!-- Placeholder for search or other actions -->
            <button class="p-1 hover:bg-black/5 dark:hover:bg-white/10 rounded-[var(--radius-vsk)]">🔍</button>
        </div>
      </div>

      <div class="flex-1 flex min-h-0">
        <!-- Sidebar -->
        <div class="w-48 p-2 border-r border-gray-200 dark:border-gray-700/50 flex flex-col gap-1 bg-white/20 dark:bg-black/10 backdrop-blur-sm">
          <button
            v-for="item in sidebarItems"
            :key="item.name"
            @click="handleSidebarClick(item)"
            class="flex items-center gap-3 px-3 py-2 rounded-[var(--radius-vsk)] hover:bg-white/40 dark:hover:bg-white/10 transition-colors text-sm font-medium text-left"
          >
            <span>{{ item.icon }}</span>
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
            <div
              v-for="file in files"
              :key="file.name"
              @dblclick="handleItemClick(file)"
              class="group flex flex-col items-center gap-2 p-3 rounded-[var(--radius-vsk)] hover:bg-blue-500/10 dark:hover:bg-blue-400/10 hover:ring-1 hover:ring-vsk-primary cursor-pointer transition-all"
            >
              <div class="text-4xl opacity-80 group-hover:scale-110 transition-transform duration-200">
                {{ file.is_dir ? '📁' : '📄' }}
              </div>
              <span class="text-xs text-center truncate w-full px-1 select-none font-medium text-gray-700 dark:text-gray-300 group-hover:text-vsk-primary" :title="file.name">
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
