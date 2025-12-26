<script setup lang="ts">
import { ref, watch, nextTick, onMounted, onUnmounted, Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { getIconSource } from '@vasakgroup/plugin-vicons'

interface SearchResult {
  id: string
  title: string
  description: string
  icon: string | null
  category: 'application' | 'file' | 'action'
  exec: string | null
  path: string | null
  score: number
}

const query = ref('')
const results = ref<SearchResult[]>([])
const selectedIndex = ref(0)
const loading = ref(false)
const currentWindow = getCurrentWindow()

// Icono del sistema para búsqueda (vicons)
const searchIconSrc: Ref<string> = ref("")

let debounceTimer: number | null = null

// Watch query changes
watch(query, (newQuery) => {
  if (debounceTimer) {
    clearTimeout(debounceTimer)
  }

  if (!newQuery.trim()) {
    results.value = []
    selectedIndex.value = 0
    return
  }

  loading.value = true
  debounceTimer = setTimeout(async () => {
    try {
      const searchResults = await invoke<SearchResult[]>('global_search', {
        query: newQuery,
        limit: 50
      })
      results.value = searchResults
      selectedIndex.value = 0
    } catch (error) {
      console.error('[search] Error:', error)
      results.value = []
    } finally {
      loading.value = false
    }
  }, 150) as unknown as number
})

// Keyboard navigation
const handleKeydown = async (event: KeyboardEvent) => {
  switch (event.key) {
    case 'Escape':
      event.preventDefault()
      await currentWindow.close()
      break

    case 'ArrowDown':
      event.preventDefault()
      if (selectedIndex.value < results.value.length - 1) {
        selectedIndex.value++
        scrollToSelected()
      }
      break

    case 'ArrowUp':
      event.preventDefault()
      if (selectedIndex.value > 0) {
        selectedIndex.value--
        scrollToSelected()
      }
      break

    case 'Enter':
      event.preventDefault()
      if (results.value[selectedIndex.value]) {
        await executeResult(results.value[selectedIndex.value])
      }
      break
  }
}

// Execute selected result
const executeResult = async (result: SearchResult) => {
  try {
    await invoke('execute_search_result', {
      id: result.id,
      category: result.category,
      exec: result.exec
    })
    await currentWindow.close()
  } catch (error) {
    console.error('[search] Execution error:', error)
  }
}

// Scroll to selected item
const scrollToSelected = () => {
  nextTick(() => {
    const selected = document.querySelector('.search-result.selected')
    if (selected) {
      selected.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
    }
  })
}

// Auto-focus input on mount
onMounted(async () => {
  // Cargar ícono de búsqueda desde vicons
  searchIconSrc.value = await getIconSource('search')
  nextTick(() => {
    document.querySelector<HTMLInputElement>('.search-input')?.focus()
  })
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})

// Helper functions
function getCategoryIcon(category: string): string {
  switch (category) {
    case 'application':
      return '📦'
    case 'file':
      return '📄'
    case 'action':
      return '⚡'
    default:
      return '🔹'
  }
}

function getCategoryLabel(category: string): string {
  switch (category) {
    case 'application':
      return 'App'
    case 'file':
      return 'Archivo'
    case 'action':
      return 'Acción'
    default:
      return ''
  }
}
</script>

<template>
  <div class="h-screen w-screen flex items-center justify-center">
    <div class="background rounded-window flex flex-col w-[700px] max-h-[80vh] overflow-hidden group relative hover:shadow-lg transition-all duration-300">
      <!-- Decorative overlay like ThemeToggle -->
      <div class="absolute inset-0 rounded-window transition-all duration-500 opacity-0 group-hover:opacity-100 z-0"></div>
      <!-- Header -->
      <div class="flex items-center gap-4 px-6 py-5 border-b border-vsk-primary/10 relative z-10">
        <img :src="searchIconSrc" alt="Search" class="w-7 h-7 flex-shrink-0" />
        <input
          v-model="query"
          type="text"
          class="search-input flex-1 bg-transparent border-none outline-none text-xl text-vsk-text font-medium placeholder:text-vsk-text/30"
          placeholder="Buscar aplicaciones, archivos y acciones..."
          autofocus
        />
        <div v-if="loading" class="text-2xl animate-spin">⏳</div>
      </div>

      <!-- Results -->
      <div v-if="results.length > 0" class="flex-1 overflow-y-auto px-3 py-3 relative z-10">
        <div
          v-for="(result, index) in results"
          :key="result.id"
          class="search-result flex items-center gap-4 p-4 rounded-vsk cursor-pointer transition-all duration-200 mb-2 ring-1 ring-vsk-primary/20"
          :class="index === selectedIndex ? 'selected bg-gradient-to-r from-vsk-primary/20 to-vsk-primary/10 translate-x-2 shadow-md shadow-vsk-primary/15 ring-2 ring-vsk-primary/40 scale-[1.01]' : 'hover:bg-gradient-to-r hover:from-vsk-primary/20 hover:to-vsk-primary/10 hover:translate-x-2 hover:shadow-md hover:shadow-vsk-primary/15 hover:ring-vsk-primary/30'"
          @click="executeResult(result)"
          @mouseenter="selectedIndex = index"
        >
          <div class="w-14 h-14 flex-shrink-0 flex items-center justify-center text-3xl rounded-vsk bg-vsk-primary/10 border border-vsk-primary/20">
            {{ getCategoryIcon(result.category) }}
          </div>
          <div class="flex-1 min-w-0">
            <div class="text-lg font-bold text-vsk-text">{{ result.title }}</div>
            <div v-if="result.description" class="text-sm text-vsk-text/60 truncate">
              {{ result.description }}
            </div>
          </div>
          <div class="px-3 py-1 rounded-full bg-vsk-primary/25 border border-vsk-primary/30">
            <span class="text-xs font-bold text-vsk-primary uppercase tracking-wider">
              {{ getCategoryLabel(result.category) }}
            </span>
          </div>
        </div>
      </div>

      <!-- Empty States -->
      <div v-else-if="query.trim() && !loading" class="flex-1 flex items-center justify-center px-6 py-12 relative z-10">
        <p class="text-center text-vsk-text/40 text-base">No se encontraron resultados</p>
      </div>

      <div v-else-if="!query.trim()" class="flex-1 flex items-center justify-center px-6 py-12 relative z-10">
        <p class="text-center text-vsk-text/40 text-base">Escribe para buscar aplicaciones, archivos o acciones</p>
      </div>

      <!-- Footer -->
      <div class="flex justify-center px-6 py-4 border-t border-vsk-primary/10 bg-black/10 relative z-10">
        <div class="flex gap-5 items-center text-xs text-vsk-text/50 font-medium">
          <span class="flex items-center gap-1">
            <kbd class="bg-vsk-primary/15 px-2 py-1 rounded border border-vsk-primary/20 text-vsk-primary font-semibold">↑↓</kbd>
            Navegar
          </span>
          <span class="flex items-center gap-1">
            <kbd class="bg-vsk-primary/15 px-2 py-1 rounded border border-vsk-primary/20 text-vsk-primary font-semibold">Enter</kbd>
            Ejecutar
          </span>
          <span class="flex items-center gap-1">
            <kbd class="bg-vsk-primary/15 px-2 py-1 rounded border border-vsk-primary/20 text-vsk-primary font-semibold">Esc</kbd>
            Cerrar
          </span>
        </div>
      </div>
    </div>
  </div>
</template>
