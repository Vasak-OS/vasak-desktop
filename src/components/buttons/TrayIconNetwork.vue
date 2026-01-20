<script lang="ts" setup>
import { ref, computed, onMounted, onUnmounted, Ref } from 'vue';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { listen } from '@tauri-apps/api/event';
import {
	getCurrentNetworkState,
	type NetworkInfo,
	WiFiSecurityType,
} from '@vasakgroup/plugin-network-manager';
import { toggleNetworkApplet } from '@/tools/network.controller';

let ulisten: Ref<Function | null> = ref(null);
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

const networkAlt = computed(() => {
	return networkState.value.is_connected
		? `Conectado a ${networkState.value.connection_type} ${networkState.value.ssid}`
		: 'Conectado a red desconocida';
});

const getCurrentNetwork = async () => {
	try {
		networkState.value = await getCurrentNetworkState();
		networkIconSrc.value = await getSymbolSource(networkState.value.icon);
		return networkState;
	} catch (error) {
		networkIconSrc.value = await getSymbolSource('network-offline-symbolic');
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
			networkIconSrc.value = await getSymbolSource(event.payload.icon);
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
  <div class="p-1 rounded-vsk relative hover:bg-vsk-primary/30" @click="toggleNetworkApplet">
    <img
      :src="networkIconSrc"
      :alt="networkAlt"
      class="m-auto h-[22px] w-auto transition-all duration-300"
      :class="{
        'filter brightness-75': !networkState.is_connected,
      }"
    />
    <div
      class="absolute top-3 right-0.5 w-3 h-3 rounded-full transition-all duration-300"
      :class="
        networkState.is_connected ? 'bg-green-400 animate-pulse' : 'bg-red-400'
      "
    ></div>
  </div>
</template>
