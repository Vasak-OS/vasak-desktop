<script setup lang="ts">
import VolumeControl from "@/components/VolumeControl.vue";
import BrightnessControl from "@/components/BrightnessControl.vue";
import NotificationArea from "@/components/areas/control-center/NotificationArea.vue";
import NetworkControl from "@/components/NetworkControl.vue";
import BluetoothControl from "@/components/BluetoothControl.vue";
import UserControlCenterCard from "@/components/cards/UserControlCenterCard.vue";
import ThemeToggle from "@/components/ThemeToggle.vue";

</script>

<template>
  <main class="bg-white/70 dark:bg-black/70 text-black dark:text-white h-screen w-screen rounded-xl flex flex-row flex-wrap justify-between">
    <div class="flex flex-col w-full gap-2 p-2">
      <UserControlCenterCard />
      <NotificationArea />
    </div>
    <div class="flex flex-wrap w-full justify-around items-end self-end p-2">
      <NetworkControl />
      <BluetoothControl />
      <ThemeToggle />
      <div class="flex flex-col gap-2 w-full mt-4">
        <BrightnessControl />
        <VolumeControl />
      </div>
    </div>
  </main>
</template>
