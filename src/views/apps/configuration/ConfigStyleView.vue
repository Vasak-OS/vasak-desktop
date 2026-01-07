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
      dark_mode: vskConfig!.value?.style?.darkmode || false,
      icon_pack: selectedIconPack.value,
      cursor_theme: selectedCursorTheme.value,
      gtk_theme: selectedGtkTheme.value,
    };

    await invoke("set_system_config", { config });
  } catch (err) {
    console.error("Error aplicando cambios del sistema:", err);
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
    <div class="p-6 max-w-[600px] mx-auto">
      <h2 class="text-2xl font-semibold mb-6 text-vsk-primary">
        Configuración de Estilos
      </h2>

      <div
        v-if="loading"
        class="flex flex-col items-center justify-center py-[60px] px-5 gap-4"
      >
        <div
          class="w-10 h-10 border-4 border-[var(--surface-2,rgba(255,255,255,0.1))] border-t-[var(--primary-color,#0084ff)] rounded-full animate-spin"
        ></div>
        <p>Cargando configuración...</p>
      </div>

      <div v-else>
        <!-- Mensajes de error/éxito -->
        <div
          v-if="error"
          class="p-3 px-4 rounded-vsk mb-4 text-sm bg-red-500/10 border border-red-500/30 text-red-400"
        >
          {{ error }}
        </div>
        <div
          v-if="successMessage"
          class="p-3 px-4 rounded-vsk mb-4 text-sm bg-green-500/10 border border-green-500/30 text-green-400"
        >
          {{ successMessage }}
        </div>

        <!-- Formulario de configuración -->
        <div class="flex flex-col gap-6">
          <!-- Sección Apariencia -->
          <div class="flex flex-col gap-4 p-4 background rounded-vsk">
            <h3 class="text-base font-semibold m-0 text-vsk-primary">
              🎨 Apariencia
            </h3>

            <!-- Border Radius -->
            <div class="flex flex-col gap-2">
              <label
                for="border-radius"
                class="text-sm font-medium text-vsk-primary flex justify-between items-center"
              >
                <span>Border Radius</span>
                <span
                  class="text-xs text-[var(--text-secondary,rgba(255,255,255,0.7))] font-normal"
                  >{{ vskConfig?.style.radius }}px</span
                >
              </label>
              <input
                v-if="vskConfig"
                id="border-radius"
                :value="vskConfig.style.radius"
                @input="e => (vskConfig!.style.radius = parseInt((e.target as HTMLInputElement).value))"
                type="range"
                min="1"
                max="20"
                class="w-full h-1.5 rounded-[3px] bg-[var(--surface-3,rgba(255,255,255,0.1))] outline-none appearance-none [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[var(--primary-color,#0084ff)] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:transition-all [&::-webkit-slider-thumb]:duration-200 [&::-webkit-slider-thumb]:shadow-[0_2px_8px_rgba(0,132,255,0.3)] hover:[&::-webkit-slider-thumb]:scale-110 [&::-moz-range-thumb]:w-[18px] [&::-moz-range-thumb]:h-[18px] [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:bg-[var(--primary-color,#0084ff)] [&::-moz-range-thumb]:cursor-pointer [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:transition-all [&::-moz-range-thumb]:duration-200 [&::-moz-range-thumb]:shadow-[0_2px_8px_rgba(0,132,255,0.3)] hover:[&::-moz-range-thumb]:scale-110"
              />
              <div
                class="flex justify-between text-xs text-[var(--text-secondary,rgba(255,255,255,0.7))]"
              >
                <span>1px</span>
                <span>20px</span>
              </div>
            </div>

            <!-- Primary Color -->
            <div class="flex flex-col gap-2">
              <label
                for="primary-color"
                class="text-sm font-medium text-vsk-primary"
                >Color Primario</label
              >
              <div class="flex gap-3 items-center">
                <input
                  v-if="vskConfig"
                  id="primary-color"
                  :value="vskConfig.style.primarycolor"
                  @input="e => (vskConfig!.style.primarycolor = (e.target as HTMLInputElement).value)"
                  type="color"
                  class="w-[50px] h-10 border-2 border-[var(--surface-3,rgba(255,255,255,0.1))] rounded-vsk cursor-pointer transition-colors duration-200 hover:border-[var(--primary-color,#0084ff)]"
                />
                <input
                  v-if="vskConfig"
                  :value="vskConfig.style.primarycolor"
                  @input="e => (vskConfig!.style.primarycolor = (e.target as HTMLInputElement).value)"
                  type="text"
                  placeholder="#0084FF"
                  class="flex-1 py-2 px-3 background rounded-vsk text-vsk-primary text-sm font-mono transition-all duration-200 focus:outline-none focus:border-[var(--primary-color,#0084ff)] focus:bg-[var(--surface-2,rgba(255,255,255,0.1))]"
                />
              </div>
            </div>

            <!-- Dark Mode Toggle -->
            <div class="flex flex-row items-center justify-between gap-2">
              <label
                for="dark-mode"
                class="text-sm font-medium text-vsk-primary"
                >Modo Oscuro</label
              >
              <div class="flex items-center gap-3">
                <input
                  v-if="vskConfig"
                  id="dark-mode"
                  :checked="vskConfig.style.darkmode"
                  @change="e => (vskConfig!.style.darkmode = (e.target as HTMLInputElement).checked)"
                  type="checkbox"
                  class="w-12 h-7 appearance-none bg-[var(--surface-3,rgba(255,255,255,0.1))] border-2 border-[var(--surface-3,rgba(255,255,255,0.2))] rounded-[14px] cursor-pointer relative transition-all duration-300 outline-none before:content-[''] before:absolute before:w-5 before:h-5 before:rounded-full before:bg-white before:left-1 before:top-0.5 before:transition-all before:duration-300 checked:bg-[var(--primary-color,#0084ff)] checked:border-[var(--primary-color,#0084ff)] checked:before:left-[22px]"
                />
                <span
                  class="text-sm text-[var(--text-secondary,rgba(255,255,255,0.7))]"
                  >{{
                    vskConfig?.style.darkmode ? "Activado" : "Desactivado"
                  }}</span
                >
              </div>
            </div>
          </div>

          <!-- Sección Tema GTK -->
          <div class="flex flex-col gap-4 p-4 background rounded-vsk">
            <h3 class="text-base font-semibold m-0 text-vsk-primary">
              🖥️ Tema GTK
            </h3>

            <div class="flex flex-col gap-2">
              <label
                for="gtk-theme"
                class="text-sm font-medium text-vsk-primary"
                >Tema GTK</label
              >
              <select
                v-model="selectedGtkTheme"
                id="gtk-theme"
                class="py-2.5 px-3 background rounded-vsk text-vsk-primary text-sm cursor-pointer transition-all duration-200 appearance-none pr-9 bg-[url('data:image/svg+xml;charset=UTF-8,%3csvg_xmlns=%27http://www.w3.org/2000/svg%27_viewBox=%270_0_24_24%27_fill=%27none%27_stroke=%27white%27_stroke-width=%272%27_stroke-linecap=%27round%27_stroke-linejoin=%27round%27%3e%3cpolyline_points=%276_9_12_15_18_9%27%3e%3c/polyline%3e%3c/svg%3e')] bg-no-repeat bg-[right_8px_center] bg-[length:20px] hover:border-[var(--primary-color,#0084ff)] hover:bg-[var(--surface-2,rgba(255,255,255,0.08))] focus:outline-none focus:border-[var(--primary-color,#0084ff)] focus:bg-[var(--surface-2,rgba(255,255,255,0.1))] focus:shadow-[0_0_0_3px_rgba(0,132,255,0.1)]"
              >
                <option
                  v-for="theme in gtkThemes"
                  :key="theme"
                  :value="theme"
                  class="background text-vsk-primary py-2 my-1"
                >
                  {{ theme }}
                </option>
              </select>
            </div>
          </div>

          <!-- Sección Cursor -->
          <div class="flex flex-col gap-4 p-4 background rounded-vsk">
            <h3 class="text-base font-semibold m-0 text-vsk-primary">
              🖱️ Cursor
            </h3>

            <div class="flex flex-col gap-2">
              <label
                for="cursor-theme"
                class="text-sm font-medium text-vsk-primary"
                >Tema de Cursor</label
              >
              <select
                v-model="selectedCursorTheme"
                id="cursor-theme"
                class="py-2.5 px-3 background rounded-vsk text-vsk-primary text-sm cursor-pointer transition-all duration-200 appearance-none pr-9 bg-[url('data:image/svg+xml;charset=UTF-8,%3csvg_xmlns=%27http://www.w3.org/2000/svg%27_viewBox=%270_0_24_24%27_fill=%27none%27_stroke=%27white%27_stroke-width=%272%27_stroke-linecap=%27round%27_stroke-linejoin=%27round%27%3e%3cpolyline_points=%276_9_12_15_18_9%27%3e%3c/polyline%3e%3c/svg%3e')] bg-no-repeat bg-[right_8px_center] bg-[length:20px] hover:border-[var(--primary-color,#0084ff)] hover:bg-[var(--surface-2,rgba(255,255,255,0.08))] focus:outline-none focus:border-[var(--primary-color,#0084ff)] focus:bg-[var(--surface-2,rgba(255,255,255,0.1))] focus:shadow-[0_0_0_3px_rgba(0,132,255,0.1)]"
              >
                <option
                  v-for="cursor in cursorThemes"
                  :key="cursor"
                  :value="cursor"
                  class="background text-vsk-primary py-2 my-1"
                >
                  {{ cursor }}
                </option>
              </select>
            </div>
          </div>

          <!-- Sección Iconos -->
          <div class="flex flex-col gap-4 p-4 background rounded-vsk">
            <h3 class="text-base font-semibold m-0 text-vsk-primary">
              🎯 Iconos
            </h3>

            <div class="flex flex-col gap-2">
              <label
                for="icon-pack"
                class="text-sm font-medium text-vsk-primary"
                >Pack de Iconos</label
              >
              <select
                v-model="selectedIconPack"
                id="icon-pack"
                class="py-2.5 px-3 background rounded-vsk text-vsk-primary text-sm cursor-pointer transition-all duration-200 appearance-none pr-9 bg-[url('data:image/svg+xml;charset=UTF-8,%3csvg_xmlns=%27http://www.w3.org/2000/svg%27_viewBox=%270_0_24_24%27_fill=%27none%27_stroke=%27white%27_stroke-width=%272%27_stroke-linecap=%27round%27_stroke-linejoin=%27round%27%3e%3cpolyline_points=%276_9_12_15_18_9%27%3e%3c/polyline%3e%3c/svg%3e')] bg-no-repeat bg-[right_8px_center] bg-[length:20px] hover:border-[var(--primary-color,#0084ff)] hover:bg-[var(--surface-2,rgba(255,255,255,0.08))] focus:outline-none focus:border-[var(--primary-color,#0084ff)] focus:bg-[var(--surface-2,rgba(255,255,255,0.1))] focus:shadow-[0_0_0_3px_rgba(0,132,255,0.1)]"
              >
                <option
                  v-for="pack in iconPacks"
                  :key="pack"
                  :value="pack"
                  class="background text-vsk-primary py-2 my-1"
                >
                  {{ pack }}
                </option>
              </select>
              <p class="text-xs text-yellow-500 m-0">
                ⚠️ Cambiar el pack de iconos requiere refrescar las aplicaciones
              </p>
            </div>
          </div>

          <!-- Botones de acción -->
          <div class="flex gap-3 mt-6">
            <button
              @click="saveConfig"
              :disabled="!isFormValid || saving"
              class="flex-1 py-3 px-6 border-0 rounded-vsk text-sm font-medium cursor-pointer transition-all duration-200 flex items-center justify-center gap-2 bg-[var(--primary-color,#0084ff)] text-white hover:opacity-90 hover:shadow-[0_4px_12px_rgba(0,132,255,0.3)] disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <span
                v-if="saving"
                class="inline-block w-4 h-4 border-2 border-transparent border-t-current rounded-full animate-spin"
              ></span>
              {{ saving ? "Guardando..." : "Guardar Cambios" }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </ConfigAppLayout>
</template>
