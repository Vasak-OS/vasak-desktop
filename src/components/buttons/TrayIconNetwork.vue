<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, onUnmounted, type Ref, ref } from 'vue';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import {
	getCurrentNetworkState,
	getVpnStatus,
	toggleNetworkApplet,
	type NetworkInfo,
	type VpnStatus,
} from '@/services/network.service';
import { logError } from '@/utils/logger';

interface Props {
	showProfileName?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
	showProfileName: true,
});

let unlistenNetwork: Ref<(() => void) | null> = ref(null);
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

const vpnLabel = computed(() => {
	if (!vpnConnected.value) return 'VPN desconectada';
	return vpnStatus.value?.active_profile_name
		? `VPN: ${vpnStatus.value.active_profile_name}`
		: 'VPN conectada';
});

const activeVpnProfileName = computed(() => {
	if (!vpnConnected.value) return '';
	return vpnStatus.value?.active_profile_name || 'Perfil VPN activo';
});

const networkAlt = computed(() => {
	const networkLabel = networkState.value.is_connected
		? `Conectado: ${networkState.value.connection_type} ${networkState.value.ssid}`
		: 'Red desconectada';
	return `${networkLabel} · ${vpnLabel.value}`;
});

const getCurrentNetwork = async () => {
	try {
		networkState.value = await getCurrentNetworkState();
		networkIconSrc.value = await getSymbolSource(networkState.value.icon);
		return networkState;
	} catch (error) {
		networkIconSrc.value = await getSymbolSource('network-offline-symbolic');
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

	unlistenNetwork.value = await listen<NetworkInfo>('network-changed', async (event) => {
		networkState.value = event.payload;
		networkIconSrc.value = await getSymbolSource(event.payload.icon);
	});

	unlistenVpn.value = await listen('vpn-changed', refreshVpnStatus);
});

onUnmounted(() => {
	unlistenNetwork.value?.();
	unlistenVpn.value?.();
});
</script>

<template>
  <div class="flex items-center gap-1">
	<TrayIconButton
	  :icon="networkIconSrc"
	  :alt="networkAlt"
	  :tooltip="networkAlt"
	  :custom-class="{ 'relative': true }"
		:icon-class="{ 'filter brightness-90': !networkState.is_connected, 'drop-shadow-[0_0_6px_rgba(59,130,246,0.5)]': vpnConnected }"
	  @click="toggleNetworkApplet"
	>
	  <div
			class="absolute top-3 right-0.5 w-2.5 h-2.5 rounded-full transition-all duration-300 ring-1 ring-ui-bg"
			:class="networkState.is_connected ? 'bg-status-success animate-pulse' : 'bg-status-danger'"
	  ></div>

		<div
			v-if="vpnConnected"
			class="absolute -top-0.5 -left-0.5 rounded-full bg-primary text-tx-on-primary text-[8px] leading-none px-1 py-0.5 font-bold border border-ui-bg"
			title="VPN activa"
		>
			VPN
		</div>
	</TrayIconButton>

	<span
		v-if="props.showProfileName && activeVpnProfileName"
		class="hidden lg:inline-block max-w-40 truncate text-xs font-medium text-tx-muted"
		:title="`Perfil VPN activo: ${activeVpnProfileName}`"
	>
		{{ activeVpnProfileName }}
	</span>
  </div>
</template>
