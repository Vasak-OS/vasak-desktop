<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, onUnmounted, type Ref, ref } from 'vue';
import {
	getCurrentNetworkState,
	getVpnStatus,
	type NetworkInfo,
	type VpnStatus,
} from '@/services/network.service';
import { logError } from '@/utils/logger';
import ToggleControl from '../forms/ToggleControl.vue';
import { toggleNetworkApplet } from '@/services/window.service';

let ulisten: Ref<(() => void) | null> = ref(null);
let unlistenVpn: Ref<(() => void) | null> = ref(null);
const networkState: Ref<NetworkInfo> = ref<NetworkInfo>({
	name: 'Unknown',
	ssid: 'Unknown',
	connection_type: 'Unknown',
	icon: 'network-offline-symbolic',
	ip_address: '0.0.0.0',
	mac_address: '00:00:00:00:00:00',
	signal_strength: 0,
	security_type: 'none',
	is_connected: false,
});
const vpnStatus: Ref<VpnStatus | null> = ref(null);
const networkIconSrc: Ref<string> = ref('');

const vpnConnected = computed(() => vpnStatus.value?.state === 'connected');

const networkAlt = computed(() => {
	const net = networkState.value.is_connected
		? `Conectado a ${networkState.value.connection_type} ${networkState.value.ssid}`
		: 'Desconectado';
	const vpn = vpnConnected.value
		? `VPN: ${vpnStatus.value?.active_profile_name || 'activa'}`
		: 'VPN inactiva';
	return `${net} · ${vpn}`;
});

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

const refreshVpnStatus = async () => {
	try {
		vpnStatus.value = await getVpnStatus();
	} catch (error) {
		vpnStatus.value = null;
		logError('Error getting VPN status:', error);
	}
};

onMounted(async () => {
	await getCurrentNetwork();
	await refreshVpnStatus();
	ulisten.value = await listen<NetworkInfo>('network-changed', async (event) => {
		networkState.value = event.payload;
		networkIconSrc.value = await getIconSource(event.payload.icon);
	});
	unlistenVpn.value = await listen('vpn-changed', refreshVpnStatus);
});

onUnmounted(() => {
	if (ulisten.value !== null) {
		ulisten.value();
	}
	if (unlistenVpn.value !== null) {
		unlistenVpn.value();
	}
});
</script>

<template>
	<div class="relative inline-block">
		<!-- Indicador de estado -->
		<div
			class="absolute top-1 right-1 w-3 h-3 rounded-full transition-all duration-300"
			:class="networkState.is_connected ? 'bg-status-success animate-pulse' : 'bg-status-error'"
		></div>

		<!-- Indicador de intensidad de señal -->
		<div v-if="networkState.is_connected" class="absolute bottom-1 left-1 flex space-x-0.5">
			<div
				v-for="i in 4"
				:key="i"
				class="w-1 bg-primary rounded-full transition-all duration-300"
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
			:custom-class="{
				'ring-2 ring-status-success': networkState.is_connected && !vpnConnected,
				'ring-2 ring-primary': vpnConnected,
				'ring-2 ring-status-error': !networkState.is_connected,
			}"
			@click="toggleNetworkApplet"
		/>
	</div>
</template>
