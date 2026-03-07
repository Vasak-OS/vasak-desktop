
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
  <div class="bg-ui-bg/80 h-screen w-screen border border-ui-border rounded-corner-window p-4">
    <BluetoothControlArea />
  </div>
</template>
