<script setup lang="ts">
import { WindowFrame } from "@vasakgroup/vue-libvasak";
import { ref, onMounted } from "vue";
import { readTextFile } from "@tauri-apps/plugin-fs";
import { join, homeDir } from "@tauri-apps/api/path";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { getIconSource } from "@vasakgroup/plugin-vicons";

// Interfaces
interface FileEntry {
  name: string;
  is_dir: boolean;
  size: string;
  path: string;
  // Metadata for UI
  icon?: string;
  previewUrl?: string;
  mimeType?: string; // specific type inference
  loadError?: boolean;
}

interface SidebarItem {
    name: string;
    icon: string;
    path: string;
}

// State
const sidebarItems = ref([] as SidebarItem[]);

const files = ref<FileEntry[]>([]);
const currentPath = ref("");
const homePath = ref("");
const loading = ref(false);
const error = ref("");
const showHidden = ref(false);
const sidebarIcons = ref<Record<string, string>>({});

// Icon Mapping Helper
const getIconNameForFile = (filename: string, isDir: boolean): string => {
  if (isDir) return "folder";
  
  const ext = filename.split('.').pop()?.toLowerCase() || "";
  
  switch(ext) {
    case "png": case "jpg": case "jpeg": case "gif": case "webp": case "svg": return "image-x-generic";
    case "mp4": case "webm": case "mkv": case "avi": return "video-x-generic";
    case "mp3": case "wav": case "flac": case "ogg": return "audio-x-generic";
    case "txt": case "md": case "log": return "text-x-generic";
    case "pdf": return "application-pdf";
    case "zip": case "tar": case "gz": case "7z": case "rar": return "package-x-generic";
    case "rs": case "ts": case "js": case "vue": case "py": case "c": case "cpp": return "text-x-script"; // or generic code icon
    case "html": case "css": return "text-html";
    case "exe": case "sh": case "bin": return "application-x-executable";
    default: return "text-x-generic"; // Fallback
  }
};

const isImage = (filename: string) => /\.(jpg|jpeg|png|gif|webp|svg)$/i.test(filename);
const isVideo = (filename: string) => /\.(mp4|webm|mkv)$/i.test(filename);

// Process entries to add icons and previews
const processEntries = async (entries: FileEntry[]) => {
  const processed: FileEntry[] = [];
  
  for (const entry of entries) {
    const item: FileEntry = { ...entry };
    
    // 1. Generate Preview for Images/Videos
    if (!item.is_dir) {
        if (isImage(item.name)) {
            item.previewUrl = convertFileSrc(item.path);
            item.mimeType = "image";
        } else if (isVideo(item.name)) {
             item.previewUrl = convertFileSrc(item.path);
             item.mimeType = "video";
        }
    }

    // 2. Fetch Icon if no preview (or as fallback/overlay)
    const iconName = getIconNameForFile(item.name, item.is_dir);
    try {
        const source = await getIconSource(iconName);
        if (source && source.startsWith('/')) {
            item.icon = convertFileSrc(source);
        } else {
             item.icon = source;
        }
    } catch (e) {
        console.warn(`Failed to load icon ${iconName}`, e);
        // Fallback or empty
    }
    
    processed.push(item);
  }
  return processed;
};

// Actions
const loadFiles = async (path: string) => {
  loading.value = true;
  error.value = "";
  files.value = []; // Clear for immediate feedback or keep old? Clear feels snappier for nav.
  
  try {
    const entries = await invoke<FileEntry[]>("read_directory", { path, showHidden: showHidden.value });
    // Post-process for icons/previews
    files.value = await processEntries(entries);
    currentPath.value = path;
  } catch (err: any) {
    error.value = err.toString();
    console.error("Error reading directory:", err);
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

const handleSidebarClick = (item: SidebarItem) => {
  // item.path is now mostly absolute, except for special HOME logic if we kept it distinct, 
  // but better to align all to absolute paths effectively.
  // Although HOME is conceptually nicer as a variable.
  
  if (item.path === "HOME") {
      navigateTo(homePath.value);
  } else if (item.path.startsWith('/')) {
      navigateTo(item.path);
  } else {
      // Fallback relative to home if path is simple name
      navigateTo(`${homePath.value}/${item.path}`);
  }
};

const loadSidebar = async () => {
    const items: SidebarItem[] = [];
    
    // 1. Always add Home
    items.push({ name: "Home", icon: "user-home", path: "HOME" });
    
    try {
        const configPath = await join(await homeDir(), '.config', 'user-dirs.dirs');
        const content = await readTextFile(configPath);
        
        // Map XDG var to icon
        const iconMap: Record<string, string> = {
            "XDG_DOCUMENTS_DIR": "folder-documents",
            "XDG_DOWNLOAD_DIR": "folder-download",
            "XDG_MUSIC_DIR": "folder-music",
            "XDG_PICTURES_DIR": "folder-pictures",
            "XDG_VIDEOS_DIR": "folder-videos",
            "XDG_DESKTOP_DIR": "user-desktop"
        };
        
        const lines = content.split('\n');
        for (const line of lines) {
            const trimmed = line.trim();
            if(!trimmed || trimmed.startsWith('#')) continue;
            
            // Format: XDG_xxx_DIR="$HOME/yyy"
            const match = trimmed.match(/^(XDG_[A-Z_]+_DIR)="(.*)"/);
            if (match) {
                const key = match[1];
                let val = match[2];
                
                // Ignored keys
                if (key === "XDG_TEMPLATES_DIR" || key === "XDG_PUBLICSHARE_DIR") continue;
                
                // Resolve $HOME
                const resolvedPath = val.replace('$HOME', homePath.value);
                // Extract clean name (basename)
                const name = resolvedPath.split('/').pop() || val;
                
                const icon = iconMap[key] || "folder";
                
                items.push({ name, icon, path: resolvedPath });
            }
        }
    } catch(e) {
        console.warn("Failed to load user-dirs.dirs, falling back to defaults", e);
        // Fallback defaults if file missing
        items.push(
            { name: "Documents", icon: "folder-documents", path: "Documents" },
            { name: "Downloads", icon: "folder-download", path: "Downloads" },
            { name: "Pictures", icon: "folder-pictures", path: "Pictures" },
            { name: "Music", icon: "folder-music", path: "Music" }
        );
    }
    
    sidebarItems.value = items;
    
    // Load icons for the dynamic items
    for (const item of sidebarItems.value) {
        try {
            const source = await getIconSource(item.icon);
            sidebarIcons.value[item.name] = source.startsWith('/') ? convertFileSrc(source) : source;
        } catch(e) { console.warn("Sidebar icon fail", item.icon) }
    }
};

onMounted(async () => {
    try {
        homePath.value = await homeDir();
        currentPath.value = homePath.value;
        
        // Load sidebar AFTER home path is set
        await loadSidebar();
        
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
        <div class="ml-4 flex gap-2 items-center">
            <!-- Hidden Files Toggle -->
            <label class="flex items-center gap-2 text-xs font-medium text-gray-600 dark:text-gray-400 cursor-pointer select-none hover:text-gray-900 dark:hover:text-gray-200">
                <input type="checkbox" :checked="showHidden" @change="toggleHiddenFiles" class="rounded border-gray-300 dark:border-gray-600 focus:ring-vsk-primary text-vsk-primary">
                Show Hidden
            </label>
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
            <!-- Use loaded icon or fallback emoji -->
            <img v-if="sidebarIcons[item.name]" :src="sidebarIcons[item.name]" class="w-5 h-5" />
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
            <div
              v-for="file in files"
              :key="file.name"
              @dblclick="handleItemClick(file)"
              class="group flex flex-col items-center gap-2 p-3 rounded-[var(--radius-vsk)] hover:bg-blue-500/10 dark:hover:bg-blue-400/10 hover:ring-1 hover:ring-vsk-primary cursor-pointer transition-all"
            >
              <!-- Icon / Preview Container -->
              <div class="w-16 h-16 flex items-center justify-center overflow-hidden rounded-[var(--radius-vsk)] bg-black/5 dark:bg-white/5 group-hover:scale-105 transition-transform duration-200">
                 
                 <!-- Image Preview -->
                 <img v-if="file.mimeType === 'image' && file.previewUrl && !file.loadError" :src="file.previewUrl" class="w-full h-full object-cover" loading="lazy" @error="file.loadError = true" />
                 
                 <!-- Video Preview (Icon fallback if needed, but video tag for now) -->
                 <video v-else-if="file.mimeType === 'video' && file.previewUrl && !file.loadError" :src="file.previewUrl" class="w-full h-full object-cover" muted loop @mouseenter="(e) => (e.target as HTMLVideoElement).play()" @mouseleave="(e) => (e.target as HTMLVideoElement).pause()" @error="file.loadError = true"></video>

                 <!-- System Icon (Fallback or for non-media OR load error) -->
                 <img v-else-if="file.icon" :src="file.icon" class="w-full h-full object-contain p-2 opacity-80" />
                 
                 <!-- Fallback Text Icon -->
                 <span v-else class="text-3xl opacity-50">{{ file.is_dir ? '📁' : '📄' }}</span>

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
