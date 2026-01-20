<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import ConfigAppLayout from '@/layouts/ConfigAppLayout.vue';

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
		const loadedConfig: SystemConfig = await invoke('get_system_config');
		config.value = loadedConfig;

		// Cargar opciones disponibles
		const [themes, cursors, icons] = await Promise.all([
			invoke<string[]>('get_gtk_themes'),
			invoke<string[]>('get_cursor_themes'),
			invoke<string[]>('get_icon_packs'),
		]);

		gtkThemes.value = themes;
		cursorThemes.value = cursors;
		iconPacks.value = icons;
	} catch (err) {
		error.value = `Error cargando configuración: ${err}`;
		console.error(err);
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
		await invoke('set_system_config', { config: config.value });

		// Aplicar CSS variables
		applyThemeToDOM();

		successMessage.value = 'Configuración guardada exitosamente';
		setTimeout(() => {
			successMessage.value = '';
		}, 3000);
	} catch (err) {
		error.value = `Error guardando configuración: ${err}`;
		console.error(err);
	} finally {
		saving.value = false;
	}
};

const applyThemeToDOM = () => {
	// Aplicar CSS variables al documento
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
    <div class="config-os-view">
      <h2 class="view-title">Configuración del Sistema Operativo</h2>

      <div v-if="loading" class="loading">
        <div class="spinner"></div>
        <p>Cargando configuración...</p>
      </div>

      <div v-else>
        <!-- Mensajes de error/éxito -->
        <div v-if="error" class="alert alert-error">{{ error }}</div>
        <div v-if="successMessage" class="alert alert-success">{{ successMessage }}</div>

        <!-- Formulario de configuración -->
        <div class="config-form">
          <!-- Sección Apariencia -->
          <div class="config-section">
            <h3 class="section-title">🎨 Apariencia</h3>

            <!-- Border Radius -->
            <div class="form-group">
              <label for="border-radius">
                <span class="label-text">Border Radius</span>
                <span class="label-value">{{ config.border_radius }}px</span>
              </label>
              <input
                id="border-radius"
                v-model.number="config.border_radius"
                type="range"
                min="1"
                max="20"
                class="slider"
              />
              <div class="slider-labels">
                <span>1px</span>
                <span>20px</span>
              </div>
            </div>

            <!-- Primary Color -->
            <div class="form-group">
              <label for="primary-color" class="label-text">Color Primario</label>
              <div class="color-input-wrapper">
                <input
                  id="primary-color"
                  v-model="config.primary_color"
                  type="color"
                  class="color-input"
                />
                <input
                  v-model="config.primary_color"
                  type="text"
                  placeholder="#0084FF"
                  class="color-text"
                />
              </div>
            </div>

            <!-- Accent Color -->
            <div class="form-group">
              <label for="accent-color" class="label-text">Color de Énfasis</label>
              <div class="color-input-wrapper">
                <input
                  id="accent-color"
                  v-model="config.accent_color"
                  type="color"
                  class="color-input"
                />
                <input
                  v-model="config.accent_color"
                  type="text"
                  placeholder="#FF6B6B"
                  class="color-text"
                />
              </div>
            </div>

            <!-- Dark Mode Toggle -->
            <div class="form-group toggle-group">
              <label for="dark-mode" class="label-text">Modo Oscuro</label>
              <div class="toggle-switch">
                <input
                  id="dark-mode"
                  v-model="config.dark_mode"
                  type="checkbox"
                  class="toggle-input"
                />
                <span class="toggle-label">{{ config.dark_mode ? 'Activado' : 'Desactivado' }}</span>
              </div>
            </div>
          </div>

          <!-- Sección Tema GTK -->
          <div class="config-section">
            <h3 class="section-title">🖥️ Tema GTK</h3>

            <div class="form-group">
              <label for="gtk-theme" class="label-text">Tema GTK</label>
              <select v-model="config.gtk_theme" id="gtk-theme" class="select-input">
                <option v-for="theme in gtkThemes" :key="theme" :value="theme">
                  {{ theme }}
                </option>
              </select>
            </div>
          </div>

          <!-- Sección Cursor -->
          <div class="config-section">
            <h3 class="section-title">🖱️ Cursor</h3>

            <div class="form-group">
              <label for="cursor-theme" class="label-text">Tema de Cursor</label>
              <select v-model="config.cursor_theme" id="cursor-theme" class="select-input">
                <option v-for="cursor in cursorThemes" :key="cursor" :value="cursor">
                  {{ cursor }}
                </option>
              </select>
            </div>
          </div>

          <!-- Sección Iconos -->
          <div class="config-section">
            <h3 class="section-title">🎯 Iconos</h3>

            <div class="form-group">
              <label for="icon-pack" class="label-text">Pack de Iconos</label>
              <select v-model="config.icon_pack" id="icon-pack" class="select-input">
                <option v-for="pack in iconPacks" :key="pack" :value="pack">
                  {{ pack }}
                </option>
              </select>
              <p class="help-text">
                ⚠️ Cambiar el pack de iconos requiere refrescar las aplicaciones
              </p>
            </div>
          </div>

          <!-- Botones de acción -->
          <div class="form-actions">
            <button
              @click="saveConfig"
              :disabled="!isFormValid || saving"
              class="btn btn-primary"
            >
              <span v-if="saving" class="spinner-small"></span>
              {{ saving ? 'Guardando...' : 'Guardar Cambios' }}
            </button>
            <button
              @click="resetToDefaults"
              :disabled="saving"
              class="btn btn-secondary"
            >
              Restablecer Valores por Defecto
            </button>
          </div>
        </div>
      </div>
    </div>
  </ConfigAppLayout>
</template>

<style scoped>
.config-os-view {
  padding: 24px;
  max-width: 600px;
  margin: 0 auto;
}

.view-title {
  font-size: 24px;
  font-weight: 600;
  margin-bottom: 24px;
  color: var(--text-primary, #fff);
}

.loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  gap: 16px;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 4px solid var(--surface-2, rgba(255, 255, 255, 0.1));
  border-top-color: var(--primary-color, #0084FF);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.spinner-small {
  display: inline-block;
  width: 16px;
  height: 16px;
  border: 2px solid transparent;
  border-top-color: currentColor;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

.alert {
  padding: 12px 16px;
  border-radius: 8px;
  margin-bottom: 16px;
  font-size: 14px;
}

.alert-error {
  color: #ff6b6b;
  background-color: rgba(255, 107, 107, 0.1);
  border: 1px solid rgba(255, 107, 107, 0.3);
}

.alert-success {
  color: #4caf50;
  background-color: rgba(76, 175, 80, 0.1);
  border: 1px solid rgba(76, 175, 80, 0.3);
}

.config-form {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.config-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 16px;
  background-color: var(--surface-2, rgba(255, 255, 255, 0.05));
  border-radius: 8px;
  border: 1px solid var(--surface-3, rgba(255, 255, 255, 0.1));
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
  color: var(--text-primary, #fff);
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.label-text {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary, #fff);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.label-value {
  font-size: 12px;
  color: var(--text-secondary, rgba(255, 255, 255, 0.7));
  font-weight: 400;
}

.slider {
  width: 100%;
  height: 6px;
  border-radius: 3px;
  background-color: var(--surface-3, rgba(255, 255, 255, 0.1));
  outline: none;
  -webkit-appearance: none;
  appearance: none;
}

.slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background-color: var(--primary-color, #0084FF);
  cursor: pointer;
  transition: all 0.2s ease;
  box-shadow: 0 2px 8px rgba(0, 132, 255, 0.3);
}

.slider::-webkit-slider-thumb:hover {
  transform: scale(1.1);
}

.slider::-moz-range-thumb {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background-color: var(--primary-color, #0084FF);
  cursor: pointer;
  border: none;
  transition: all 0.2s ease;
  box-shadow: 0 2px 8px rgba(0, 132, 255, 0.3);
}

.slider::-moz-range-thumb:hover {
  transform: scale(1.1);
}

.slider-labels {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: var(--text-secondary, rgba(255, 255, 255, 0.7));
}

.color-input-wrapper {
  display: flex;
  gap: 12px;
  align-items: center;
}

.color-input {
  width: 50px;
  height: 40px;
  border: 2px solid var(--surface-3, rgba(255, 255, 255, 0.1));
  border-radius: 6px;
  cursor: pointer;
  transition: border-color 0.2s ease;
}

.color-input:hover {
  border-color: var(--primary-color, #0084FF);
}

.color-text {
  flex: 1;
  padding: 8px 12px;
  background-color: var(--surface-3, rgba(255, 255, 255, 0.05));
  border: 1px solid var(--surface-3, rgba(255, 255, 255, 0.1));
  border-radius: 6px;
  color: var(--text-primary, #fff);
  font-size: 14px;
  font-family: monospace;
  transition: all 0.2s ease;
}

.color-text:focus {
  outline: none;
  border-color: var(--primary-color, #0084FF);
  background-color: var(--surface-2, rgba(255, 255, 255, 0.1));
}

.select-input {
  padding: 10px 12px;
  background-color: var(--surface-3, rgba(255, 255, 255, 0.05));
  border: 1px solid var(--surface-3, rgba(255, 255, 255, 0.1));
  border-radius: 6px;
  color: var(--text-primary, #fff);
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.select-input:hover {
  border-color: var(--primary-color, #0084FF);
}

.select-input:focus {
  outline: none;
  border-color: var(--primary-color, #0084FF);
  background-color: var(--surface-2, rgba(255, 255, 255, 0.1));
}

.select-input option {
  background-color: var(--surface-1, #1a1a1a);
  color: var(--text-primary, #fff);
}

.help-text {
  font-size: 12px;
  color: var(--text-secondary, rgba(255, 255, 255, 0.7));
  margin: 0;
}

.toggle-group {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
}

.toggle-switch {
  display: flex;
  align-items: center;
  gap: 12px;
}

.toggle-input {
  width: 48px;
  height: 28px;
  appearance: none;
  -webkit-appearance: none;
  background-color: var(--surface-3, rgba(255, 255, 255, 0.1));
  border: 2px solid var(--surface-3, rgba(255, 255, 255, 0.2));
  border-radius: 14px;
  cursor: pointer;
  position: relative;
  transition: all 0.3s ease;
  outline: none;
}

.toggle-input:checked {
  background-color: var(--primary-color, #0084FF);
  border-color: var(--primary-color, #0084FF);
}

.toggle-input::before {
  content: '';
  position: absolute;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background-color: white;
  left: 4px;
  top: 2px;
  transition: all 0.3s ease;
}

.toggle-input:checked::before {
  left: 22px;
}

.toggle-label {
  font-size: 14px;
  color: var(--text-secondary, rgba(255, 255, 255, 0.7));
}

.form-actions {
  display: flex;
  gap: 12px;
  margin-top: 24px;
}

.btn {
  padding: 12px 24px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  flex: 1;
}

.btn-primary {
  background-color: var(--primary-color, #0084FF);
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background-color: var(--primary-color, #0084FF);
  opacity: 0.9;
  box-shadow: 0 4px 12px rgba(0, 132, 255, 0.3);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-secondary {
  background-color: var(--surface-3, rgba(255, 255, 255, 0.1));
  color: var(--text-primary, #fff);
  border: 1px solid var(--surface-3, rgba(255, 255, 255, 0.2));
}

.btn-secondary:hover:not(:disabled) {
  background-color: var(--surface-2, rgba(255, 255, 255, 0.15));
  border-color: var(--text-primary, #fff);
}

.btn-secondary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
