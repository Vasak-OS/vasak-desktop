<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import {
	getCurrentNetworkState,
	type NetworkInfo,
	toggleNetwork,
	WiFiSecurityType,
} from '@vasakgroup/plugin-network-manager';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, onUnmounted, type Ref, ref } from 'vue';
import { ToggleControl } from '@vasakgroup/vue-libvasak';
import { logError } from '@/utils/logger';

let ulisten: Ref<(() => void) | null> = ref(null);
const networkState: Ref<NetworkInfo> = ref<NetworkInfo>({
	name: 'Unknown',
	ssid: 'Unknown',
	connection_type: 'Unknown',
	icon: 'network-offline-symbolic',
	ip_address: '0.0.0.0',
	mac_address: '00:00:00:00:00:00',
	signal_strength: 0,
	security_type: WiFiSecurityType.NONE,
	is_connected: false,
});
const networkIconSrc: Ref<string> = ref('');
const isLoading: Ref<boolean> = ref(false);

const networkAlt = computed(() => {
	return networkState.value.is_connected
		? `Conectado a ${networkState.value.connection_type} ${networkState.value.ssid}`
		: 'Desconectado';
});

const toggleCurrentNetwork = async (): Promise<void> => {
	isLoading.value = true;
	try {
		await toggleNetwork(!networkState.value.is_connected);
	} catch (error) {
		logError('Error toggling network:', error);
	} finally {
		isLoading.value = false;
	}
};

const getCurrentNetwork = async () => {
	try {
		networkState.value = await getCurrentNetworkState();
		networkIconSrc.value = await getIconSource(networkState.value.icon);
		return networkState;
	} catch (error) {
		networkIconSrc.value = await getIconSource('network-offline-symbolic');
		logError('Error getting current network state:', error);
		return null;
	}
};

onMounted(async () => {
	await getCurrentNetwork();
	ulisten.value = await listen<NetworkInfo>('network-changed', async (event) => {
		networkState.value = event.payload;
		networkIconSrc.value = await getIconSource(event.payload.icon);
	});
});

onUnmounted(() => {
	if (ulisten.value !== null) {
		ulisten.value();
	}
});
</script>

<template>
	<div class="relative inline-block">
		<!-- Indicador de estado -->
		<div
			class="absolute top-1 right-1 w-3 h-3 rounded-full transition-all duration-300"
			:class="networkState.is_connected ? 'bg-green-400 animate-pulse' : 'bg-red-400'"
		></div>

		<!-- Indicador de intensidad de señal -->
		<div v-if="networkState.is_connected" class="absolute bottom-1 left-1 flex space-x-0.5">
			<div
				v-for="i in 4"
				:key="i"
				class="w-1 bg-vsk-primary rounded-full transition-all duration-300"
				:class="{
					'opacity-100': i <= Math.ceil(networkState.signal_strength / 25),
					'opacity-30': i > Math.ceil(networkState.signal_strength / 25),
				}"
				:style="{ height: `${4 + i * 2}px` }"
			></div>
		</div>

		<ToggleControl
			:icon="networkIconSrc"
			:alt="networkAlt"
			:tooltip="networkAlt"
			:is-active="networkState.is_connected"
			:is-loading="isLoading"
			:custom-class="{
				'ring-2 ring-green-400/50': networkState.is_connected,
				'ring-2 ring-red-400/50': !networkState.is_connected,
			}"
			@click="toggleCurrentNetwork"
		/>
	</div>
</template>
