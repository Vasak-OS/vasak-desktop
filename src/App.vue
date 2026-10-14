<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: imports used in template */

import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { onMounted, ref } from 'vue';
import { RouterView } from 'vue-router';
import { useSharedEvent } from '@/tools/event.bus';
import { viewTransitionGuard } from '@/tools/view.transition';
import { logError, logInfo } from '@/utils/logger';

// Start loading config immediately, don't wait for onMounted
const configLoading = ref(true);
const configError = ref(false);
const configPromise = (async () => {
	try {
		const configStore = useConfigStore();
		await configStore.loadConfig();
		configLoading.value = false;
		logInfo('Configuración cargada correctamente');
	} catch (error: any) {
		configLoading.value = false;
		configError.value = true;
		logError('Error al cargar configuración en App.vue', { error: error.message });
	}
})();

onMounted(async () => {
	// Ensure config loading started before mount
	await configPromise;

	// Config changes handling
	useSharedEvent('config-changed', (payload: any) => {
		logInfo('Evento config-changed recibido, recargando configuración');
		const configStore = useConfigStore();

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
});
</script>

<template>
  <div v-if="configLoading" class="config-skeleton">
    <div class="animate-pulse space-y-4 p-4">
      <div class="h-8 bg-ui-line animate-pulse" style="border-radius: var(--radius-corner-m); width: 75%;" />
      <div class="h-8 bg-ui-line animate-pulse" style="border-radius: var(--radius-corner-m); width: 50%;" />
      <div class="h-8 bg-ui-line animate-pulse" style="border-radius: var(--radius-corner-m); width: 25%;" />
    </div>
  </div>
  <div v-else-if="configError" class="error-banner">
    Error al cargar configuración. Usando valores por defecto.
  </div>
  <RouterView />
</template>
