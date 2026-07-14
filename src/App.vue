<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: imports used in template */
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import type { Store } from 'pinia';
import { onMounted } from 'vue';
import { RouterView } from 'vue-router';
import { useSharedEvent } from '@/tools/event.bus';
import { viewTransitionGuard } from '@/tools/view.transition';
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

useSharedEvent('config-changed', (payload: any) => {
	logInfo('Evento config-changed recibido, recargando configuración');
	const configStore = useConfigStore() as Store<
		'config',
		{ config: any; loadConfig: () => Promise<void> }
	>;

	// Only use View Transition for user-initiated theme switches
	if (payload?.key === 'theme' || payload?.type === 'theme') {
		viewTransitionGuard.startTransition(() => configStore.loadConfig());
		return;
	}

	// Non-theme config changes: defer during active transition, otherwise execute immediately
	viewTransitionGuard.deferUpdate(() => {
		configStore.loadConfig();
	});
});
</script>

<template>
  <RouterView />
</template>
