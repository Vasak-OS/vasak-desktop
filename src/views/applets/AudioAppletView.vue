
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
  <div class="applet background h-screen w-screen">
    <SystemAudioArea />
  </div>
</template>
