<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: imports used in template */
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import type { Store } from 'pinia';
import { onMounted } from 'vue';
import { RouterView } from 'vue-router';
import { useEventListener } from '@/tools/event.listener';
import { logDebug, logError, logInfo } from '@/utils/logger';

onMounted(async () => {
	logInfo('App.vue montado, cargando configuración');
	try {
		const configStore = useConfigStore() as Store<
			'config',
			{ config: any; loadConfig: () => Promise<void> }
		>;
		await configStore.loadConfig();
		logDebug('Configuración cargada correctamente');
	} catch (error: any) {
		logError('Error al cargar configuración en App.vue', { error: error.message });
	}
});

useEventListener('config-changed', () => {
	logInfo('Evento config-changed recibido, recargando configuración');
	const configStore = useConfigStore() as Store<
		'config',
		{ config: any; loadConfig: () => Promise<void> }
	>;
	if (typeof document.startViewTransition === 'function') {
		return document.startViewTransition(() => configStore.loadConfig());
	}
	return configStore.loadConfig();
});
</script>

<template>
  <RouterView />
</template>
