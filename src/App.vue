<script setup lang="ts">
import { RouterView } from "vue-router";
import { useConfigStore } from "@vasakgroup/plugin-config-manager";
import { Store } from 'pinia';
import { listen, emit } from "@tauri-apps/api/event";
import { onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";

let unlistenConfig: Function | null = null;
let unlistenShortcuts: (Function | null)[] = [];
const router = useRouter();

const handleVasakShortcuts = async () => {
  // Listener para búsqueda (Super+Space)
  const unlistenSearch = await listen("shortcut:toggle_search", () => {
    console.log("Shortcut: Toggle Search");
    emit("vasak:toggle_search", {});
  });

  // Listener para menú (Super)
  const unlistenMenu = await listen("shortcut:toggle_menu", () => {
    console.log("Shortcut: Toggle Menu");
    emit("vasak:toggle_menu", {});
  });

  // Listener para control center (Super+C)
  const unlistenControlCenter = await listen("shortcut:toggle_control_center", () => {
    console.log("Shortcut: Toggle Control Center");
    emit("vasak:toggle_control_center", {});
  });

  // Listener para configuración (Super+,)
  const unlistenConfig = await listen("shortcut:open_configuration_window", () => {
    console.log("Shortcut: Open Configuration");
    router.push("/apps/configuration/shortcuts");
  });

  unlistenShortcuts = [unlistenSearch, unlistenMenu, unlistenControlCenter, unlistenConfig];
};

onMounted(async () => {
  const configStore = useConfigStore() as Store<"config", { config: any; loadConfig: () => Promise<void>; }>;
  await configStore.loadConfig();
  unlistenConfig = await listen("config-changed", async () => {
    document.startViewTransition(() => {
      configStore.loadConfig();
    });
  });

  // Configurar listeners para atajos de VasakOS
  await handleVasakShortcuts();
});

onUnmounted(() => {
  if (unlistenConfig !== null) {
    unlistenConfig();
  }
  unlistenShortcuts.forEach(fn => {
    if (fn !== null) fn();
  });
});
</script>

<template>
  <RouterView />
</template>
