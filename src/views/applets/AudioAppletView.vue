<script lang="ts" setup>
import { invoke } from '@tauri-apps/api/core';
import { onMounted } from 'vue';
import { logError } from '@/utils/logger';

onMounted(async () => {
	document.addEventListener('keydown', (event) => {
		if (event.key === 'Escape') {
			try {
				invoke('toggle_audio_applet');
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
