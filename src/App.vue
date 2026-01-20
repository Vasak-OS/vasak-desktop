<script setup lang="ts">
import { RouterView } from 'vue-router';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { Store } from 'pinia';
import { listen } from '@tauri-apps/api/event';
import { onMounted, onUnmounted } from 'vue';

let unlistenConfig: Function | null = null;

onMounted(async () => {
	const configStore = useConfigStore() as Store<
    'config',
    { config: any; loadConfig: () => Promise<void> }
  >;
	await configStore.loadConfig();
	unlistenConfig = await listen('config-changed', async () => {
		document.startViewTransition(() => {
			configStore.loadConfig();
		});
	});
});

onUnmounted(() => {
	if (unlistenConfig !== null) {
		unlistenConfig();
	}
});
</script>

<template>
  <RouterView />
</template>
