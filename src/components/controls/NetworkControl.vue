<template>
  <button
    @click="toggleCurrentNetwork"
    class="p-2 rounded-vsk background hover:opacity-50 transition-all duration-300 h-17.5 w-17.5 group relative overflow-hidden hover:scale-105 hover:shadow-lg active:scale-95"
    :class="{
      'animate-pulse': isLoading,
      'ring-2 ring-green-400/50': networkState.is_connected,
      'ring-2 ring-red-400/50': !networkState.is_connected,
    }"
    :disabled="isLoading"
  >
    <!-- Background glow effect -->
    <div
      class="absolute inset-0 rounded-vsk bg-lineal-to-br from-blue-500/20 to-cyan-500/20 opacity-0 group-hover:opacity-100 transition-opacity duration-300"
    ></div>

    <!-- Connection status indicator -->
    <div
      class="absolute top-1 right-1 w-3 h-3 rounded-full transition-all duration-300"
      :class="
        networkState.is_connected ? 'bg-green-400 animate-pulse' : 'bg-red-400'
      "
    ></div>

    <!-- Signal strength indicator -->
    <div
      v-if="networkState.is_connected"
      class="absolute bottom-1 left-1 flex space-x-0.5"
    >
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

    <img
      :src="networkIconSrc"
      :alt="networkAlt"
      :title="networkAlt"
      class="m-auto w-12.5 h-12.5 transition-all duration-300 group-hover:scale-110 relative z-10"
      :class="{
        'animate-spin': isLoading,
        'opacity-60': !networkState.is_connected,
        'drop-shadow-lg': networkState.is_connected,
      }"
    />
  </button>
</template>

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
		: 'Conectado a red desconocida';
});

const toggleCurrentNetwork = async () => {
	if (isLoading.value) return;

	isLoading.value = true;
	try {
		await toggleNetwork(!networkState.value.is_connected);
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
	ulisten.value = await listen<NetworkInfo>('network-changed', async (event) => {
		console.log('Network changed', event.payload);
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
