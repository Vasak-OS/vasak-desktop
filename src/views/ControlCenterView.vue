<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { isBluetoothPluginInitialized } from '@vasakgroup/plugin-bluetooth-manager';
import { onMounted, type Ref, ref } from 'vue';
import NotificationArea from '@/components/areas/control-center/NotificationArea.vue';
import UserControlCenterCard from '@/components/cards/UserControlCenterCard.vue';
import BluetoothControl from '@/components/controls/BluetoothControl.vue';
import BrightnessControl from '@/components/controls/BrightnessControl.vue';
import NetworkControl from '@/components/controls/NetworkControl.vue';
import SearchButtonControl from '@/components/controls/SearchButtonControl.vue';
import ThemeToggle from '@/components/controls/ThemeToggle.vue';
import VolumeControl from '@/components/controls/VolumeControl.vue';
import MusicWidget from '@/components/widgets/MusicWidget.vue';
import { logError } from '@/utils/logger';

const bluetoothInitialized: Ref<boolean> = ref(false);

onMounted(async () => {
	bluetoothInitialized.value = await isBluetoothPluginInitialized();
	document.addEventListener('keydown', (event) => {
		if (event.key === 'Escape') {
			try {
				invoke('toggle_control_center');
			} catch (error) {
				logError('Error al cerrar:', error);
			}
		}
	});
});
</script>

<template>
  <main
    class="background h-screen w-screen rounded-window flex flex-row flex-wrap justify-between p-2"
  >
    <div class="flex flex-col w-full gap-2 p-2">
      <UserControlCenterCard />
      <NotificationArea />
    </div>
    <div class="flex flex-wrap w-full justify-around items-end self-end p-2">
      <MusicWidget class="w-full" />
      <div class="flex gap-2 w-full">
        <SearchButtonControl />
        <NetworkControl />
        <BluetoothControl v-if="bluetoothInitialized" />
        <ThemeToggle />
      </div>
      <div class="flex flex-col gap-2 w-full mt-4">
        <BrightnessControl />
        <VolumeControl />
      </div>
    </div>
  </main>
</template>
