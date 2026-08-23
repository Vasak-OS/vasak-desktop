<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import {
	getCurrentNetworkState,
	getVpnStatus,
	type NetworkInfo,
	toggleNetworkApplet,
	type VpnStatus,
} from '@/services/network.service';
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const networkState = ref<NetworkInfo>({
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
const vpnStatus = ref<VpnStatus | null>(null);
const networkIconSrc = useSymbol(computed(() => networkState.value.icon));

const vpnConnected = computed(() => vpnStatus.value?.state === 'connected');

const vpnLabel = computed(() => {
	if (!vpnConnected.value) return t('components.TrayIconNetwork.vpnDisconnected');
	return vpnStatus.value?.active_profile_name
		? `VPN: ${vpnStatus.value.active_profile_name}`
		: t('components.TrayIconNetwork.vpnConnected');
});

const networkAlt = computed(() => {
	const networkLabel = networkState.value.is_connected
		? t('components.TrayIconNetwork.connectedTo')
				.replace('{0}', String(networkState.value.connection_type))
				.replace('{1}', String(networkState.value.ssid))
		: t('components.TrayIconNetwork.networkDisconnected');
	return `${networkLabel} · ${vpnLabel.value}`;
});

const getCurrentNetwork = async () => {
	try {
		networkState.value = await getCurrentNetworkState();
		return networkState;
	} catch (error) {
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
});

useSharedEvent<NetworkInfo>('network-changed', (payload) => {
	networkState.value = payload;
});

useSharedEvent('vpn-changed', refreshVpnStatus);
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
			:class="networkState.is_connected ? 'bg-status-success animate-pulse' : 'bg-status-error'"
	  ></div>

	</TrayIconButton>

  </div>
</template>
