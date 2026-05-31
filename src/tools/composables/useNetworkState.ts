import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, ref } from 'vue';
import {
	getCurrentNetworkState,
	getVpnStatus,
	type NetworkInfo,
	type VpnStatus,
} from '@/services/network.service';
import { useEventListener } from '@/tools/event.listener';
import { logError } from '@/utils/logger';

export function useNetworkState() {
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
	const networkIconSrc = ref('');

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

	useEventListener<NetworkInfo>('network-changed', async (event) => {
		networkState.value = event.payload;
		networkIconSrc.value = await getIconSource(event.payload.icon);
	});

	useEventListener('vpn-changed', refreshVpnStatus);

	return {
		networkState,
		vpnStatus,
		networkIconSrc,
		vpnConnected,
		networkAlt,
		getCurrentNetwork,
		refreshVpnStatus,
	};
}
