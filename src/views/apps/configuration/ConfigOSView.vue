<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { computed, onMounted, ref } from 'vue';
import ActionButton from '@/components/buttons/ActionButton.vue';
import ConfigAppLayout from '@/components/layouts/ConfigAppLayout.vue';
import { getCursorThemes, getGtkThemes, getIconPacks } from '@/services/core.service';
import { getSystemConfig, setSystemConfig } from '@/services/system.service';
import { logError } from '@/utils/logger';

interface SystemConfig {
	border_radius: number;
	primary_color: string;
	accent_color: string;
	dark_mode: boolean;
	icon_pack: string;
	cursor_theme: string;
	gtk_theme: string;
}

const config = ref<SystemConfig>({
	border_radius: 8,
	primary_color: '#0084FF',
	accent_color: '#FF6B6B',
	dark_mode: false,
	icon_pack: 'Adwaita',
	cursor_theme: 'Adwaita',
	gtk_theme: 'Adwaita',
});

const gtkThemes = ref<string[]>([]);
const cursorThemes = ref<string[]>([]);
const iconPacks = ref<string[]>([]);
const loading = ref(true);
const saving = ref(false);
const error = ref('');
const successMessage = ref('');

onMounted(async () => {
	try {
		// Cargar configuración actual
		const loadedConfig: SystemConfig = await getSystemConfig();
		config.value = loadedConfig;

		// Cargar opciones disponibles
		const [themes, cursors, icons] = await Promise.all([
			getGtkThemes(),
			getCursorThemes(),
			getIconPacks(),
		]);

		gtkThemes.value = themes;
		cursorThemes.value = cursors;
		iconPacks.value = icons;
	} catch (err) {
		error.value = `Error cargando configuración: ${err}`;
		logError(error.value);
	} finally {
		loading.value = false;
	}
});

const saveConfig = async () => {
	saving.value = true;
	error.value = '';
	successMessage.value = '';

	try {
		// Validar border radius
		if (config.value.border_radius < 1 || config.value.border_radius > 20) {
			throw new Error('Border radius debe estar entre 1 y 20');
		}

		// Guardar configuración
		await setSystemConfig({ config: config.value });

		// Aplicar CSS variables
		applyThemeToDOM();

		successMessage.value = 'Configuración guardada exitosamente';
		setTimeout(() => {
			successMessage.value = '';
		}, 3000);
	} catch (err) {
		error.value = `Error guardando configuración: ${err}`;
		logError(error.value);
	} finally {
		saving.value = false;
	}
};

const applyThemeToDOM = () => {
	const root = document.documentElement;
	root.style.setProperty('--border-radius', `${config.value.border_radius}px`);
	root.style.setProperty('--primary-color', config.value.primary_color);
	root.style.setProperty('--accent-color', config.value.accent_color);

	// Aplicar clase para modo oscuro
	if (config.value.dark_mode) {
		document.documentElement.classList.add('dark-mode');
	} else {
		document.documentElement.classList.remove('dark-mode');
	}
};

const resetToDefaults = async () => {
	if (confirm('¿Estás seguro de que deseas restablecer a los valores por defecto?')) {
		config.value = {
			border_radius: 8,
			primary_color: '#0084FF',
			accent_color: '#FF6B6B',
			dark_mode: false,
			icon_pack: 'Adwaita',
			cursor_theme: 'Adwaita',
			gtk_theme: 'Adwaita',
		};
		applyThemeToDOM();
		await saveConfig();
	}
};

const isFormValid = computed(() => {
	return (
		config.value.border_radius >= 1 &&
		config.value.border_radius <= 20 &&
		config.value.primary_color &&
		config.value.accent_color &&
		config.value.gtk_theme &&
		config.value.cursor_theme &&
		config.value.icon_pack
	);
});
</script>

<template>
  <ConfigAppLayout>
    <div class="p-6 max-w-[600px] mx-auto">
      <h2 class="text-2xl font-semibold mb-6 text-[var(--text-primary,#fff)]">Configuración del Sistema Operativo</h2>

      <div v-if="loading" class="flex flex-col items-center justify-center py-15 px-5 gap-4">
        <div class="w-10 h-10 border-4 border-[var(--surface-2,rgba(255,255,255,0.1))] border-t-[var(--primary-color,#0084FF)] rounded-full animate-spin"></div>
        <p>Cargando configuración...</p>
      </div>

      <div v-else>
        <!-- Mensajes de error/éxito -->
        <div v-if="error" class="p-3 rounded-lg mb-4 text-sm text-[#ff6b6b] bg-[rgba(255,107,107,0.1)] border border-[rgba(255,107,107,0.3)]">{{ error }}</div>
        <div v-if="successMessage" class="p-3 rounded-lg mb-4 text-sm text-[#4caf50] bg-[rgba(76,175,80,0.1)] border border-[rgba(76,175,80,0.3)]">{{ successMessage }}</div>

        <!-- Formulario de configuración -->
        <div class="flex flex-col gap-6">
          <!-- Sección Apariencia -->
          <div class="flex flex-col gap-4 p-4 bg-[var(--surface-2,rgba(255,255,255,0.05))] rounded-lg border border-[var(--surface-3,rgba(255,255,255,0.1))]">
            <h3 class="text-base font-semibold m-0 text-[var(--text-primary,#fff)]">🎨 Apariencia</h3>

            <!-- Border Radius -->
            <div class="flex flex-col gap-2">
              <label for="border-radius">
                <span class="text-sm font-medium text-[var(--text-primary,#fff)] flex justify-between items-center">Border Radius</span>
                <span class="text-xs text-[var(--text-secondary,rgba(255,255,255,0.7))] font-normal">{{ config.border_radius }}px</span>
              </label>
              <input
                id="border-radius"
                v-model.number="config.border_radius"
                type="range"
                min="1"
                max="20"
                class="w-full h-[6px] rounded-full bg-[var(--surface-3,rgba(255,255,255,0.1))] outline-none appearance-none [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[var(--primary-color,#0084FF)] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:transition-all [&::-webkit-slider-thumb]:duration-200 [&::-webkit-slider-thumb]:shadow-[0_2px_8px_rgba(0,132,255,0.3)] hover:[&::-webkit-slider-thumb]:scale-110"
              />
              <div class="w-full h-[6px] rounded-full bg-[var(--surface-3,rgba(255,255,255,0.1))] outline-none appearance-none [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[var(--primary-color,#0084FF)] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:transition-all [&::-webkit-slider-thumb]:duration-200 [&::-webkit-slider-thumb]:shadow-[0_2px_8px_rgba(0,132,255,0.3)] hover:[&::-webkit-slider-thumb]:scale-110-labels">
                <span>1px</span>
                <span>20px</span>
              </div>
            </div>

            <!-- Primary Color -->
            <div class="flex flex-col gap-2">
              <label for="primary-color" class="text-sm font-medium text-[var(--text-primary,#fff)] flex justify-between items-center">Color Primario</label>
              <div class="flex gap-3 items-center">
                <input
                  id="primary-color"
                  v-model="config.primary_color"
                  type="color"
                  class="w-[50px] h-[40px] border-2 border-[var(--surface-3,rgba(255,255,255,0.1))] rounded-md cursor-pointer transition-colors duration-200 hover:border-[var(--primary-color,#0084FF)]"
                />
                <input
                  v-model="config.primary_color"
                  type="text"
                  placeholder="#0084FF"
                  class="flex-1 py-2 px-3 bg-[var(--surface-3,rgba(255,255,255,0.05))] border border-[var(--surface-3,rgba(255,255,255,0.1))] rounded-md text-[var(--text-primary,#fff)] text-sm font-mono transition-all duration-200 focus:outline-none focus:border-[var(--primary-color,#0084FF)] focus:bg-[var(--surface-2,rgba(255,255,255,0.1))]"
                />
              </div>
            </div>

            <!-- Accent Color -->
            <div class="flex flex-col gap-2">
              <label for="accent-color" class="text-sm font-medium text-[var(--text-primary,#fff)] flex justify-between items-center">Color de Énfasis</label>
              <div class="flex gap-3 items-center">
                <input
                  id="accent-color"
                  v-model="config.accent_color"
                  type="color"
                  class="w-[50px] h-[40px] border-2 border-[var(--surface-3,rgba(255,255,255,0.1))] rounded-md cursor-pointer transition-colors duration-200 hover:border-[var(--primary-color,#0084FF)]"
                />
                <input
                  v-model="config.accent_color"
                  type="text"
                  placeholder="#FF6B6B"
                  class="flex-1 py-2 px-3 bg-[var(--surface-3,rgba(255,255,255,0.05))] border border-[var(--surface-3,rgba(255,255,255,0.1))] rounded-md text-[var(--text-primary,#fff)] text-sm font-mono transition-all duration-200 focus:outline-none focus:border-[var(--primary-color,#0084FF)] focus:bg-[var(--surface-2,rgba(255,255,255,0.1))]"
                />
              </div>
            </div>

            <!-- Dark Mode Toggle -->
            <div class="form-group flex-row items-center justify-between">
              <label for="dark-mode" class="text-sm font-medium text-[var(--text-primary,#fff)] flex justify-between items-center">Modo Oscuro</label>
              <div class="flex items-center gap-3">
                <input
                  id="dark-mode"
                  v-model="config.dark_mode"
                  type="checkbox"
                  class="w-12 h-7 appearance-none bg-[var(--surface-3,rgba(255,255,255,0.1))] border-2 border-[var(--surface-3,rgba(255,255,255,0.2))] rounded-full cursor-pointer relative transition-all duration-300 outline-none checked:bg-[var(--primary-color,#0084FF)] checked:border-[var(--primary-color,#0084FF)] before:content-[\"\"] before:absolute before:w-5 before:h-5 before:rounded-full before:bg-white before:left-1 before:top-[2px] before:transition-all before:duration-300 checked:before:left-[22px]"
                />
                <span class="text-sm text-[var(--text-secondary,rgba(255,255,255,0.7))]">{{ config.dark_mode ? 'Activado' : 'Desactivado' }}</span>
              </div>
            </div>
          </div>

          <!-- Sección Tema GTK -->
          <div class="flex flex-col gap-4 p-4 bg-[var(--surface-2,rgba(255,255,255,0.05))] rounded-lg border border-[var(--surface-3,rgba(255,255,255,0.1))]">
            <h3 class="text-base font-semibold m-0 text-[var(--text-primary,#fff)]">🖥️ Tema GTK</h3>

            <div class="flex flex-col gap-2">
              <label for="gtk-theme" class="text-sm font-medium text-[var(--text-primary,#fff)] flex justify-between items-center">Tema GTK</label>
              <select v-model="config.gtk_theme" id="gtk-theme" class="py-2.5 px-3 bg-[var(--surface-3,rgba(255,255,255,0.05))] border border-[var(--surface-3,rgba(255,255,255,0.1))] rounded-md text-[var(--text-primary,#fff)] text-sm cursor-pointer transition-all duration-200 hover:border-[var(--primary-color,#0084FF)] focus:outline-none focus:border-[var(--primary-color,#0084FF)] focus:bg-[var(--surface-2,rgba(255,255,255,0.1))] [&>option]:bg-[var(--surface-1,#1a1a1a)] [&>option]:text-[var(--text-primary,#fff)]">
                <option v-for="theme in gtkThemes" :key="theme" :value="theme">
                  {{ theme }}
                </option>
              </select>
            </div>
          </div>

          <!-- Sección Cursor -->
          <div class="flex flex-col gap-4 p-4 bg-[var(--surface-2,rgba(255,255,255,0.05))] rounded-lg border border-[var(--surface-3,rgba(255,255,255,0.1))]">
            <h3 class="text-base font-semibold m-0 text-[var(--text-primary,#fff)]">🖱️ Cursor</h3>

            <div class="flex flex-col gap-2">
              <label for="cursor-theme" class="text-sm font-medium text-[var(--text-primary,#fff)] flex justify-between items-center">Tema de Cursor</label>
              <select v-model="config.cursor_theme" id="cursor-theme" class="py-2.5 px-3 bg-[var(--surface-3,rgba(255,255,255,0.05))] border border-[var(--surface-3,rgba(255,255,255,0.1))] rounded-md text-[var(--text-primary,#fff)] text-sm cursor-pointer transition-all duration-200 hover:border-[var(--primary-color,#0084FF)] focus:outline-none focus:border-[var(--primary-color,#0084FF)] focus:bg-[var(--surface-2,rgba(255,255,255,0.1))] [&>option]:bg-[var(--surface-1,#1a1a1a)] [&>option]:text-[var(--text-primary,#fff)]">
                <option v-for="cursor in cursorThemes" :key="cursor" :value="cursor">
                  {{ cursor }}
                </option>
              </select>
            </div>
          </div>

          <!-- Sección Iconos -->
          <div class="flex flex-col gap-4 p-4 bg-[var(--surface-2,rgba(255,255,255,0.05))] rounded-lg border border-[var(--surface-3,rgba(255,255,255,0.1))]">
            <h3 class="text-base font-semibold m-0 text-[var(--text-primary,#fff)]">🎯 Iconos</h3>

            <div class="flex flex-col gap-2">
              <label for="icon-pack" class="text-sm font-medium text-[var(--text-primary,#fff)] flex justify-between items-center">Pack de Iconos</label>
              <select v-model="config.icon_pack" id="icon-pack" class="py-2.5 px-3 bg-[var(--surface-3,rgba(255,255,255,0.05))] border border-[var(--surface-3,rgba(255,255,255,0.1))] rounded-md text-[var(--text-primary,#fff)] text-sm cursor-pointer transition-all duration-200 hover:border-[var(--primary-color,#0084FF)] focus:outline-none focus:border-[var(--primary-color,#0084FF)] focus:bg-[var(--surface-2,rgba(255,255,255,0.1))] [&>option]:bg-[var(--surface-1,#1a1a1a)] [&>option]:text-[var(--text-primary,#fff)]">
                <option v-for="pack in iconPacks" :key="pack" :value="pack">
                  {{ pack }}
                </option>
              </select>
              <p class="text-xs text-[var(--text-secondary,rgba(255,255,255,0.7))] m-0">
                ⚠️ Cambiar el pack de iconos requiere refrescar las aplicaciones
              </p>
            </div>
          </div>

          <!-- Botones de acción -->
          <div class="flex gap-3 mt-6">
            <ActionButton
              label="Guardar Cambios"
              :loading="saving"
              :disabled="!isFormValid"
              @click="saveConfig"
            />
            <ActionButton
              label="Restablecer Valores por Defecto"
              variant="secondary"
              :disabled="saving"
              @click="resetToDefaults"
            />
          </div>
        </div>
      </div>
    </div>
  </ConfigAppLayout>
</template>

