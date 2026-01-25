<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, Ref } from 'vue';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { listen } from '@tauri-apps/api/event';
import {
	getCurrentNetworkState,
	type NetworkInfo,
	toggleNetwork,
	WiFiSecurityType,
} from '@vasakgroup/plugin-network-manager';
import ToggleControl from '@/components/base/ToggleControl.vue';

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
		console.log('Network toggled successfully');
	} catch (error) {
		console.error('Error toggling network:', error);
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
		console.error('Error getting current network state:', error);
		return null;
	}
};

onMounted(async () => {
	await getCurrentNetwork();
	ulisten.value = await listen<NetworkInfo>(
		'network-changed',
		async (event) => {
			networkState.value = event.payload;
			networkIconSrc.value = await getIconSource(event.payload.icon);
		}
	);
});

onUnmounted(() => {
	if (ulisten.value !== null) {
		ulisten.value();
	}
});
</script>

<template>
  <ToggleControl
    :icon="networkIconSrc"
    :alt="networkAlt"
    :tooltip="networkAlt"
    :is-active="networkState.is_connected"
    :is-loading="isLoading"
    :show-signal-strength="networkState.is_connected"
    :signal-strength="networkState.signal_strength"
    :status-indicator-class="networkState.is_connected ? 'bg-green-400 animate-pulse' : 'bg-red-400'"
    :custom-class="{
      'ring-2 ring-green-400/50': networkState.is_connected,
      'ring-2 ring-red-400/50': !networkState.is_connected,
    }"
    glow-class="from-blue-500/20 to-cyan-500/20"
    @click="toggleCurrentNetwork"
  />
</template>
