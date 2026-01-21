<script lang="ts" setup>
import { ref, onMounted, onUnmounted, computed, type Ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';
import {
	useConfigStore,
	type VSKConfig,
	readConfig,
	writeConfig,
} from '@vasakgroup/plugin-config-manager';
import ConfigAppLayout from '@/layouts/ConfigAppLayout.vue';
import { Store } from 'pinia';

const configStore = ref<any>(null);
const loading = ref(true);
const saving = ref(false);
const error = ref('');
const successMessage = ref('');

const vskConfig: Ref<VSKConfig | null> = ref(null);
const showFiles = ref(false);
const showHiddenFiles = ref(false);
const iconSize = ref(64);
const wallpaper = ref('');
let unlistenFileDrop: (() => void) | null = null;

onMounted(async () => {
	try {
		// Cargar config store
		configStore.value = useConfigStore() as Store<
      'config',
      { config: VSKConfig; loadConfig: () => Promise<void> }
    >;

		await configStore.value.loadConfig();

		// Cargar configuración actual
		vskConfig.value = await readConfig();

		// Cargar valores del desktop
		if (vskConfig.value?.desktop) {
			showFiles.value = vskConfig.value.desktop.showfiles ?? false;
			showHiddenFiles.value = vskConfig.value.desktop.showhiddenfiles ?? false;
			iconSize.value = Number(vskConfig.value.desktop.iconsize ?? 64);
			wallpaper.value = vskConfig.value.desktop.wallpaper?.[0] || '';
		}

		// Escuchar cambios de configuración desde otras vistas
		await listen('config-changed', async () => {
			vskConfig.value = await readConfig();
			if (vskConfig.value?.desktop) {
				showFiles.value = vskConfig.value.desktop.showfiles ?? false;
				showHiddenFiles.value = vskConfig.value.desktop.showhiddenfiles ?? false;
				iconSize.value = Number(vskConfig.value.desktop.iconsize ?? 64);
				wallpaper.value = vskConfig.value.desktop.wallpaper?.[0] || '';
			}
		});

		// Escuchar eventos de drag-drop de archivos en Tauri v2
		unlistenFileDrop = await listen<{ paths: string[]; position: { x: number; y: number } }>('tauri://drag-drop', (event) => {
			const paths = event.payload.paths;
			if (paths && paths.length > 0) {
				handleDropZone(paths[0]);
			}
		});


	} catch (err) {
		error.value = `Error cargando configuración: ${err}`;
		console.error(err);
	} finally {
		loading.value = false;
	}
});

onUnmounted(() => {
	if (unlistenFileDrop) {
		unlistenFileDrop();
	}
});

const saveConfig = async (): Promise<void> => {
	saving.value = true;
	error.value = '';
	successMessage.value = '';

	try {
		// Validar tamaño de icono
		if (iconSize.value < 24 || iconSize.value > 128) {
			throw new Error('El tamaño del icono debe estar entre 32 y 256');
		}

		// Actualizar config
		if (vskConfig.value) {
			vskConfig.value.desktop = {
				...vskConfig.value.desktop,
				showfiles: showFiles.value,
				showhiddenfiles: showHiddenFiles.value,
				iconsize: Number(iconSize.value),
				wallpaper: wallpaper.value ? [wallpaper.value] : [],
			};

			await writeConfig(vskConfig.value);

			successMessage.value = 'Configuración del escritorio guardada exitosamente';
			setTimeout(() => {
				successMessage.value = '';
			}, 3000);
		}
	} catch (err) {
		error.value = `Error guardando configuración: ${err}`;
		console.error(err);
	} finally {
		saving.value = false;
	}
};

const isFormValid = computed(() => {
	return iconSize.value >= 24 && iconSize.value <= 128;
});

const handleDropZone = (filePath: string) => {
	const imageExts = ['.jpg', '.jpeg', '.png', '.webp', '.bmp', '.gif'];
	if (imageExts.some(e => filePath.toLowerCase().endsWith(e))) {
		wallpaper.value = filePath;
	} else {
		error.value = 'El archivo seleccionado no es una imagen válida';
	}
};


const wallpaperPreviewUrl = computed(() => {
	if (!wallpaper.value) return '';
	return convertFileSrc(wallpaper.value);
});
</script>

<template>
  <ConfigAppLayout>
    <div class="p-6 max-w-200 mx-auto">
      <h2 class="text-2xl font-semibold mb-6 text-vsk-primary">
        Configuración del Escritorio
      </h2>

      <div v-if="loading" class="flex flex-col items-center justify-center py-15 px-5 gap-4">
        <div class="w-10 h-10 border-4 border-t-vsk-primary rounded-full animate-spin">
        </div>
        <p>Cargando configuración...</p>
      </div>

      <div v-else>
        <!-- Mensajes de error/éxito -->
        <div v-if="error" class="p-3 px-4 rounded-vsk mb-4 text-sm bg-red-500/10 border border-red-500/30 text-red-400">
          {{ error }}
        </div>
        <div v-if="successMessage"
          class="p-3 px-4 rounded-vsk mb-4 text-sm bg-green-500/10 border border-green-500/30 text-green-400">
          {{ successMessage }}
        </div>

        <!-- Formulario de configuración -->
        <div class="flex flex-col gap-6">
          <!-- Sección Fondo de Pantalla -->
          <div class="flex flex-col gap-4 p-4 background rounded-vsk">
            <h3 class="text-base font-semibold m-0 text-vsk-primary">
              🖼️ Fondo de Pantalla
            </h3>

            <!-- Preview y drag-drop area -->
            <div
              class="flex items-center justify-center w-full h-40 rounded-vsk border-2 border-dashed border-vsk-primary/30 background hover:border-vsk-primary/50 hover:bg-vsk-primary/5 transition-colors relative overflow-hidden">

              <!-- Background image si existe -->
              <img v-if="wallpaper" :src="wallpaperPreviewUrl" alt="Wallpaper preview"
                class="absolute inset-0 w-full h-full object-cover pointer-events-none"
                @error="() => console.error('Error loading wallpaper image')" />

              <!-- Overlay y texto -->
              <div
                class="absolute inset-0 flex items-center justify-center bg-black/30 hover:bg-black/20 transition-colors">
                <div class="text-center pointer-events-none">
                  <span class="text-sm block mb-2 text-white">📂 Arrastra una imagen aquí para cambiar</span>
                  <span class="text-xs text-white/70" v-if="wallpaper">{{ wallpaper.split('/').pop() }}</span>
                </div>
              </div>
            </div>

          </div>

          <!-- Sección Archivos del Escritorio -->
          <div class="flex flex-col gap-4 p-4 background rounded-vsk">
            <h3 class="text-base font-semibold m-0 text-vsk-primary">
              📁 Archivos del Escritorio
            </h3>

            <!-- Mostrar Archivos -->
            <div class="flex flex-row items-center justify-between gap-2">
              <label for="show-files" class="text-sm font-medium text-vsk-primary">Mostrar Archivos</label>
              <div class="flex items-center gap-3">
                <input id="show-files" v-model="showFiles" type="checkbox"
                  class="w-12 h-7 appearance-none background rounded-vsk cursor-pointer relative transition-all duration-300 outline-none before:content-[''] before:absolute before:w-5 before:h-5 before:rounded-full before:bg-white before:left-1 before:top-0.5 before:transition-all before:duration-300 checked:bg-vsk-primary checked:border-vsk-primary checked:before:left-5.5" />
                <span class="text-sm ">
                  {{ showFiles ? "Activado" : "Desactivado" }}
                </span>
              </div>
            </div>

            <!-- Mostrar Archivos Ocultos -->
            <div class="flex flex-row items-center justify-between gap-2">
              <label for="show-hidden" class="text-sm font-medium text-vsk-primary">Mostrar Archivos Ocultos</label>
              <div class="flex items-center gap-3">
                <input id="show-hidden" v-model="showHiddenFiles" type="checkbox" :disabled="!showFiles"
                  class="w-12 h-7 appearance-none background rounded-vsk cursor-pointer relative transition-all duration-300 outline-none before:content-[''] before:absolute before:w-5 before:h-5 before:rounded-full before:bg-white before:left-1 before:top-0.5 before:transition-all before:duration-300 checked:bg-vsk-primary checked:border-vsk-primary checked:before:left-5.5 disabled:opacity-50 disabled:cursor-not-allowed" />
                <span class="text-sm ">
                  {{ showHiddenFiles ? "Activado" : "Desactivado" }}
                </span>
              </div>
            </div>
          </div>

          <!-- Sección Tamaño de Icono -->
          <div class="flex flex-col gap-4 p-4 background rounded-vsk">
            <h3 class="text-base font-semibold m-0 text-vsk-primary">
              📐 Tamaño de Icono
            </h3>

            <div class="flex flex-col gap-2">
              <label for="icon-size" class="text-sm font-medium text-vsk-primary flex justify-between items-center">
                <span>Tamaño</span>
                <span class="text-xs  font-normal">{{ iconSize }}px</span>
              </label>
              <input id="icon-size" v-model.number="iconSize" type="range" min="24" max="128"
                class="w-full h-1.5 rounded-vsk background outline-none appearance-none [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4.5 [&::-webkit-slider-thumb]:h-4.5 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-vsk-primary [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:transition-all [&::-webkit-slider-thumb]:duration-200 [&::-webkit-slider-thumb]:shadow-[0_2px_8px_rgba(0,132,255,0.3)] hover:[&::-webkit-slider-thumb]:scale-110 [&::-moz-range-thumb]:w-4.5 [&::-moz-range-thumb]:h-4.5 [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:bg-vsk-primary [&::-moz-range-thumb]:cursor-pointer [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:transition-all [&::-moz-range-thumb]:duration-200 [&::-moz-range-thumb]:shadow-[0_2px_8px_rgba(0,132,255,0.3)] hover:[&::-moz-range-thumb]:scale-110" />
              <div class="flex justify-between text-xs ">
                <span>24px (Pequeño)</span>
                <span>128px (Grande)</span>
              </div>
            </div>
          </div>

          <!-- Botones de acción -->
          <div class="flex gap-3 mt-6">
            <button @click="saveConfig" :disabled="!isFormValid || saving"
              class="flex-1 py-3 px-6 border-0 rounded-vsk text-sm font-medium cursor-pointer transition-all duration-200 flex items-center justify-center gap-2 bg-vsk-primary text-white hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed">
              <span v-if="saving"
                class="inline-block w-4 h-4 border-2 border-transparent border-t-current rounded-full animate-spin"></span>
              {{ saving ? "Guardando..." : "Guardar Cambios" }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </ConfigAppLayout>
</template>
