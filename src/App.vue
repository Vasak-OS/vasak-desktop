<script setup lang="ts">
import { RouterView } from 'vue-router';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { Store } from 'pinia';
import { listen } from '@tauri-apps/api/event';
import { onMounted, onUnmounted } from 'vue';
import { logInfo, logError, logDebug } from '@/utils/logger';

let unlistenConfig: (() => void) | null = null;

onMounted(async () => {
	logInfo('App.vue montado, cargando configuración');
	try {
		const configStore = useConfigStore() as Store<
			'config',
			{ config: any; loadConfig: () => Promise<void> }
		>;
		await configStore.loadConfig();
		logDebug('Configuración cargada correctamente');
		
		unlistenConfig = await listen('config-changed', async () => {
			logInfo('Evento config-changed recibido, recargando configuración');
			document.startViewTransition(() => {
				configStore.loadConfig();
			});
		});
	} catch (error: any) {
		logError('Error al cargar configuración en App.vue', { error: error.message });
	}
});

onUnmounted(() => {
	logDebug('App.vue desmontado');
	if (unlistenConfig !== null) {
		unlistenConfig();
	}
});
</script>

<template>
  <RouterView />
</template>
