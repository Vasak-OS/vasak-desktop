<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: imports used in template */

import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { nextTick, onMounted, ref } from 'vue';
import { RouterView } from 'vue-router';
import { useSharedEvent } from '@/tools/event.bus';
import { viewTransitionGuard } from '@/tools/view.transition';
import { logError, logInfo } from '@/utils/logger';

// La primera lectura de la configuración va después del primer dibujo: en
// `onMounted` sin esperar, el layout se pinta con los valores por omisión y la
// configuración llega encima. Esperando a `nextTick`, la ventana ya está en
// pantalla cuando empieza la lectura, y si falla se avisa con el banner y se
// siguen usando los valores por defecto — una ventana con colores por omisión
// sigue siendo usable; una ventana que no monta no.
const configLoading = ref(true);
const configError = ref(false);

onMounted(async () => {
	try {
		// Que el layout se pinte primero, antes de cualquier lectura.
		await nextTick();

		const configStore = useConfigStore();
		await configStore.loadConfig();

		configLoading.value = false;
		logInfo('Configuración cargada correctamente');
	} catch (error: any) {
		configLoading.value = false;
		configError.value = true;
		logError('Error al cargar configuración en App.vue', { error: error.message });
	}
});

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
</script>

<template>
  <div v-if="configLoading" class="config-status">
    Cargando configuración…
  </div>
  <div v-else-if="configError" class="error-banner">
    Error al cargar configuración. Usando valores por defecto.
  </div>
  <RouterView />
</template>
