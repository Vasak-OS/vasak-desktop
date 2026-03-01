
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
import { onMounted } from 'vue';
import BluetoothControlArea from '@/components/areas/bluetooth/BluetoothControlArea.vue';
import { toggleBluetoothApplet } from '@/services/window.service';
import { logError } from '@/utils/logger';

onMounted(async () => {
	document.addEventListener('keydown', (event) => {
		if (event.key === 'Escape') {
			try {
				toggleBluetoothApplet();
			} catch (error) {
				logError('Error al cerrar applet de Bluetooth:', error);
			}
		}
	});
});
</script>

<template>
  <div class="applet background h-screen w-screen">
    <BluetoothControlArea />
  </div>
</template>
