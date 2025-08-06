<script setup lang="ts">
import { onMounted, ref, type Ref } from "vue";
import { Command } from "@tauri-apps/plugin-shell";
import WindowsArea from "@/components/areas/panel/WindowsArea.vue";
import TrayBarArea from "@/components/areas/panel/TrayBarArea.vue";
import PanelClockwidget from "@/components/widgets/PanelClockwidget.vue";
import { getIconSource } from "@vasakgroup/plugin-vicons";

const menuIcon: Ref<string> = ref("");
const notifyIcon: Ref<string> = ref("");

const setMenuIcon = async () => {
  try {
    menuIcon.value = await getIconSource("menu-editor");
  } catch (err) {
    console.error("Error: finding icon menu");
  }
};

const setNotifyIcon = async () => {
  try {
    notifyIcon.value = await getIconSource("preferences-desktop-notification");
  } catch (err) {
    console.error("Error: finding notify icon");
  }
};

const openMenu = () => {
  Command.create("vmenu").execute();
};

const openNotificationCenter = () => {
  Command.create("vasak-control-center").execute();
};

onMounted(async () => {
  setMenuIcon();
  setNotifyIcon();
});
</script>

<template>
  <nav class="vpanel background">
    <img :src="menuIcon" alt="Menu" @click="openMenu" class="app-icon" />
    <WindowsArea />
    <div class="flex content-center items-center">
      <TrayBarArea />
      <PanelClockwidget />
      <img
        :src="notifyIcon"
        alt="Menu"
        @click="openNotificationCenter"
        class="app-icon"
      />
    </div>
  </nav>
</template>

<style>
@reference "../style.css";

.vpanel {
  @apply flex justify-between items-center mx-1 hover:bg-white/80 hover:dark:bg-black/80;
  height: 30px;
  padding: 2px;
  border-radius: calc(var(--border-radius) + 2px);
}

.vpanel .app-icon {
  @apply h-6 w-6 cursor-pointer p-0.5 rounded-vsk hover:bg-vsk-primary/30 transform hover:scale-110 active:scale-95 ease-in-out;
}
</style>
