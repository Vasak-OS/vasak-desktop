
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
import { onMounted } from 'vue';
import NetworkControlArea from '@/components/areas/network/NetworkControlArea.vue';
import { toggleNetworkApplet } from '@/services/window.service';
import { logError } from '@/utils/logger';

onMounted(async () => {
	document.addEventListener('keydown', (event) => {
		if (event.key === 'Escape') {
			try {
				toggleNetworkApplet();
			} catch (error) {
				logError('Error al cerrar applet de red:', error);
			}
		}
	});
});
</script>

<template>
  <div class="applet background h-screen w-screen">
    <NetworkControlArea />
  </div>
</template>