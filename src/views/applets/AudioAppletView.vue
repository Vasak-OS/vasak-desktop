
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
import { onMounted } from 'vue';
import SystemAudioArea from '@/components/areas/audio/SystemAudioArea.vue';
import { toggleAudioApplet } from '@/services/window.service';
import { logError } from '@/utils/logger';

onMounted(async () => {
	document.addEventListener('keydown', (event) => {
		if (event.key === 'Escape') {
			try {
				toggleAudioApplet();
			} catch (error) {
				logError('Error al cerrar applet de audio:', error);
			}
		}
	});
});
</script>

<template>
  <div class="bg-ui-bg/80 h-screen w-screen border border-ui-border rounded-corner-window p-4">
    <SystemAudioArea />
  </div>
</template>
