<script lang="ts" setup>
import { ref, onMounted, computed, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  useConfigStore,
  setDarkMode,
  type VSKConfig,
  readConfig,
  writeConfig,
} from "@vasakgroup/plugin-config-manager";
import ConfigAppLayout from "@/layouts/ConfigAppLayout.vue";
import { Store } from "pinia";

const configStore = ref<any>(null);
const gtkThemes = ref<string[]>([]);
const cursorThemes = ref<string[]>([]);
const iconPacks = ref<string[]>([]);
const loading = ref(true);
const saving = ref(false);
const error = ref("");
const successMessage = ref("");

const vskConfig: Ref<VSKConfig | null> = ref(null);
const selectedGtkTheme = ref("Adwaita");
const selectedCursorTheme = ref("Adwaita");
const selectedIconPack = ref("Adwaita");

onMounted(async () => {
  try {
    // Cargar config store
    configStore.value = useConfigStore() as Store<
      "config",
      { config: VSKConfig; loadConfig: () => Promise<void> }
    >;

    await configStore.value.loadConfig();

    // Cargar valores actuales del config store para borderRadius y primaryColor
    vskConfig.value = await readConfig();

    // Cargar estado actual real del sistema (GTK, cursor, icons)
    try {
      const systemState = await invoke<{
        gtk_theme: string;
        cursor_theme: string;
        icon_pack: string;
        dark_mode: boolean;
      }>("get_current_system_state");

      selectedGtkTheme.value = systemState.gtk_theme;
      selectedCursorTheme.value = systemState.cursor_theme;
      selectedIconPack.value = systemState.icon_pack;
      // Preferir darkMode del config store ya que es el canonical
    } catch (err) {
      console.warn(
        "No se pudo obtener estado del sistema, usando valores por defecto:",
        err
      );
      selectedGtkTheme.value = "Adwaita";
      selectedCursorTheme.value = "Adwaita";
      selectedIconPack.value = "Adwaita";
    }

    // Cargar opciones disponibles
    const [themes, cursors, icons] = await Promise.all([
      invoke<string[]>("get_gtk_themes"),
      invoke<string[]>("get_cursor_themes"),
      invoke<string[]>("get_icon_packs"),
    ]);

    gtkThemes.value = themes;
    cursorThemes.value = cursors;
    iconPacks.value = icons;

    // Asegurar que el estado actual esté presente en los desplegables
    if (
      selectedGtkTheme.value &&
      !gtkThemes.value.includes(selectedGtkTheme.value)
    ) {
      gtkThemes.value.unshift(selectedGtkTheme.value);
    }
    if (
      selectedCursorTheme.value &&
      !cursorThemes.value.includes(selectedCursorTheme.value)
    ) {
      cursorThemes.value.unshift(selectedCursorTheme.value);
    }
    if (
      selectedIconPack.value &&
      !iconPacks.value.includes(selectedIconPack.value)
    ) {
      iconPacks.value.unshift(selectedIconPack.value);
    }
  } catch (err) {
    error.value = `Error cargando configuración: ${err}`;
    console.error(err);
  } finally {
    loading.value = false;
  }
});

const saveConfig = async () => {
  saving.value = true;
  error.value = "";
  successMessage.value = "";

  try {
    // Validar border radius
    if (
      vskConfig.value?.style?.radius! < 1 ||
      vskConfig.value?.style?.radius! > 20
    ) {
      throw new Error("Border radius debe estar entre 1 y 20");
    }

    // Actualizar modo oscuro via plugin
    if (
      vskConfig.value?.style?.darkmode !==
      (configStore.value.config?.style?.darkmode || false)
    ) {
      await setDarkMode(vskConfig.value?.style?.darkmode || false);
    }

    await writeConfig(vskConfig.value!);

    // Aplicar cambios del sistema (GTK, cursor, icons) via backend
    await applySystemChanges();

    successMessage.value = "Configuración guardada exitosamente";
    setTimeout(() => {
      successMessage.value = "";
    }, 3000);
  } catch (err) {
    error.value = `Error guardando configuración: ${err}`;
    console.error(err);
  } finally {
    saving.value = false;
  }
};

const applySystemChanges = async () => {
  try {
    const config = {
      icon_pack: selectedIconPack.value,
      cursor_theme: selectedCursorTheme.value,
      gtk_theme: selectedGtkTheme.value,
    };

    await invoke("set_system_config", { config });
  } catch (err) {
    console.warn("Error aplicando cambios del sistema:", err);
    // No lanzar error, permitir que continúe aunque falle el backend
  }
};

const isFormValid = computed(() => {
  return (
    selectedGtkTheme.value &&
    selectedCursorTheme.value &&
    selectedIconPack.value
  );
});
</script>

<template>
  <ConfigAppLayout>
    <div class="config-style-view">
      <h2 class="view-title">Configuración de Estilos</h2>

      <div v-if="loading" class="loading">
        <div class="spinner"></div>
        <p>Cargando configuración...</p>
      </div>

      <div v-else>
        <!-- Mensajes de error/éxito -->
        <div v-if="error" class="alert alert-error">{{ error }}</div>
        <div v-if="successMessage" class="alert alert-success">
          {{ successMessage }}
        </div>

        <!-- Formulario de configuración -->
        <div class="config-form">
          <!-- Sección Apariencia -->
          <div class="config-section">
            <h3 class="section-title">🎨 Apariencia</h3>

            <!-- Border Radius -->
            <div class="form-group">
              <label for="border-radius">
                <span class="label-text">Border Radius</span>
                <span class="label-value">{{ vskConfig?.style.radius }}px</span>
              </label>
              <input
                v-if="vskConfig"
                id="border-radius"
                :value="vskConfig.style.radius"
                @input="e => (vskConfig!.style.radius = parseInt((e.target as HTMLInputElement).value))"
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
              <label for="primary-color" class="label-text"
                >Color Primario</label
              >
              <div class="color-input-wrapper">
                <input
                  v-if="vskConfig"
                  id="primary-color"
                  :value="vskConfig.style.primarycolor"
                  @input="e => (vskConfig!.style.primarycolor = (e.target as HTMLInputElement).value)"
                  type="color"
                  class="color-input"
                />
                <input
                  v-if="vskConfig"
                  :value="vskConfig.style.primarycolor"
                  @input="e => (vskConfig!.style.primarycolor = (e.target as HTMLInputElement).value)"
                  type="text"
                  placeholder="#0084FF"
                  class="color-text"
                />
              </div>
            </div>

            <!-- Dark Mode Toggle -->
            <div class="form-group toggle-group">
              <label for="dark-mode" class="label-text">Modo Oscuro</label>
              <div class="toggle-switch">
                <input
                  v-if="vskConfig"
                  id="dark-mode"
                  :checked="vskConfig.style.darkmode"
                  @change="e => (vskConfig!.style.darkmode = (e.target as HTMLInputElement).checked)"
                  type="checkbox"
                  class="toggle-input"
                />
                <span class="toggle-label">{{
                  vskConfig?.style.darkmode ? "Activado" : "Desactivado"
                }}</span>
              </div>
            </div>
          </div>

          <!-- Sección Tema GTK -->
          <div class="config-section">
            <h3 class="section-title">🖥️ Tema GTK</h3>

            <div class="form-group">
              <label for="gtk-theme" class="label-text">Tema GTK</label>
              <select
                v-model="selectedGtkTheme"
                id="gtk-theme"
                class="select-input"
              >
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
              <label for="cursor-theme" class="label-text"
                >Tema de Cursor</label
              >
              <select
                v-model="selectedCursorTheme"
                id="cursor-theme"
                class="select-input"
              >
                <option
                  v-for="cursor in cursorThemes"
                  :key="cursor"
                  :value="cursor"
                >
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
              <select
                v-model="selectedIconPack"
                id="icon-pack"
                class="select-input"
              >
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
              {{ saving ? "Guardando..." : "Guardar Cambios" }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </ConfigAppLayout>
</template>

<style scoped>
.config-style-view {
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
  border-top-color: var(--primary-color, #0084ff);
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
  background-color: rgba(255, 107, 107, 0.1);
  border: 1px solid rgba(255, 107, 107, 0.3);
  color: #ff6b6b;
}

.alert-success {
  background-color: rgba(76, 175, 80, 0.1);
  border: 1px solid rgba(76, 175, 80, 0.3);
  color: #4caf50;
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
  background-color: var(--primary-color, #0084ff);
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
  background-color: var(--primary-color, #0084ff);
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
  border-color: var(--primary-color, #0084ff);
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
  border-color: var(--primary-color, #0084ff);
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
  appearance: none;
  -webkit-appearance: none;
  -moz-appearance: none;
  background-image: url("data:image/svg+xml;charset=UTF-8,%3csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='white' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3e%3cpolyline points='6 9 12 15 18 9'%3e%3c/polyline%3e%3c/svg%3e");
  background-repeat: no-repeat;
  background-position: right 8px center;
  background-size: 20px;
  padding-right: 36px;
}

.select-input:hover {
  border-color: var(--primary-color, #0084ff);
  background-color: var(--surface-2, rgba(255, 255, 255, 0.08));
}

.select-input:focus {
  outline: none;
  border-color: var(--primary-color, #0084ff);
  background-color: var(--surface-2, rgba(255, 255, 255, 0.1));
  box-shadow: 0 0 0 3px rgba(0, 132, 255, 0.1);
}

.select-input option {
  background-color: var(--surface-1, #1a1a1a);
  color: var(--text-primary, #fff);
  padding: 8px;
  margin: 4px 0;
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
  background-color: var(--primary-color, #0084ff);
  border-color: var(--primary-color, #0084ff);
}

.toggle-input::before {
  content: "";
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
  background-color: var(--primary-color, #0084ff);
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background-color: var(--primary-color, #0084ff);
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

<style scoped>
.config-style-view {
  padding: 24px;
  max-width: 600px;
}

.view-title {
  font-size: 24px;
  font-weight: 600;
  margin-bottom: 16px;
  color: var(--text-primary, #fff);
}

.description {
  font-size: 16px;
  line-height: 1.6;
  color: var(--text-secondary, rgba(255, 255, 255, 0.8));
  margin-bottom: 12px;
}

.description.secondary {
  font-size: 14px;
  opacity: 0.9;
}

.description strong {
  color: var(--primary-color, #0084ff);
  font-weight: 600;
}

.description a {
  color: var(--primary-color, #0084ff);
  text-decoration: none;
  transition: opacity 0.2s ease;
}

.description a:hover {
  opacity: 0.8;
  text-decoration: underline;
}
</style>
