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

	<div
		v-if="props.showProfileName && activeVpnProfileName"
		class="hidden lg:inline-flex items-center gap-1 max-w-44 truncate text-xs font-medium text-tx-muted rounded-corner border border-ui-border bg-ui-surface/40 px-2 py-1"
		:title="`Perfil VPN activo: ${activeVpnProfileName}`"
	>
		<svg class="w-3.5 h-3.5 shrink-0 text-primary" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
			<path d="M12 1L3 5V11C3 16.55 6.84 21.74 12 23C17.16 21.74 21 16.55 21 11V5L12 1M12 7C13.4 7 14.8 8.6 14.8 10V11H16V18H8V11H9.2V10C9.2 8.6 10.6 7 12 7M12 8.2C11.2 8.2 10.4 8.7 10.4 10V11H13.6V10C13.6 8.7 12.8 8.2 12 8.2Z" />
		</svg>
		<span class="truncate">{{ activeVpnProfileName }}</span>
	</div>
  </div>
</template>
