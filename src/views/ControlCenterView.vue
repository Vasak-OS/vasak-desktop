<script setup lang="ts">
import VolumeControl from "@/components/controls/VolumeControl.vue";
import BrightnessControl from "@/components/controls/BrightnessControl.vue";
import NotificationArea from "@/components/areas/control-center/NotificationArea.vue";
import NetworkControl from "@/components/controls/NetworkControl.vue";
import BluetoothControl from "@/components/controls/BluetoothControl.vue";
import UserControlCenterCard from "@/components/cards/UserControlCenterCard.vue";
import ThemeToggle from "@/components/controls/ThemeToggle.vue";
import { onMounted, Ref, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { isBluetoothPluginInitialized } from "@vasakgroup/plugin-bluetooth-manager";

const bluetoothInitialized: Ref<boolean> = ref(false);

onMounted(async () => {
  bluetoothInitialized.value = await isBluetoothPluginInitialized();
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      try {
        invoke("toggle_control_center");
      } catch (error) {
        console.error("Error al cerrar:", error);
      }
    }
  });
});
</script>

<template>
  <main
    class="bg-white/70 dark:bg-black/70 text-black dark:text-white h-screen w-screen rounded-window flex flex-row flex-wrap justify-between"
  >
    <div class="flex flex-col w-full gap-2 p-2">
      <UserControlCenterCard />
      <NotificationArea />
    </div>
    <div class="flex flex-wrap w-full justify-around items-end self-end p-2">
      <NetworkControl />
      <BluetoothControl v-if="bluetoothInitialized" />
      <ThemeToggle />
      <div class="flex flex-col gap-2 w-full mt-4">
        <BrightnessControl />
        <VolumeControl />
      </div>
    </div>
  </main>
</template>
