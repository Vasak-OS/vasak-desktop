import { invoke } from '@tauri-apps/api/core';

export type WiFiSecurityType =
	| 'none'
	| 'wep'
	| 'wpa-psk'
	| 'wpa-eap'
	| 'wpa2-psk'
	| 'wpa3-psk';

export interface NetworkInfo {
	name: string;
	ssid: string;
	connection_type: string;
	icon: string;
	ip_address: string;
	mac_address: string;
	signal_strength: number;
	security_type: WiFiSecurityType;
	is_connected: boolean;
}

export type VpnConnectionState =
	| 'disconnected'
	| 'connecting'
	| 'connected'
	| 'disconnecting'
	| 'failed'
	| 'unknown';

export interface VpnStatus {
	state: VpnConnectionState;
	active_profile_id?: string;
	active_profile_uuid?: string;
	active_profile_name?: string;
	ip_address?: string;
	gateway?: string;
	since_unix_ms?: number;
}

export interface WiFiConnectionConfig {
	ssid: string;
	password?: string;
	security_type: WiFiSecurityType;
	username?: string;
}

export const getCurrentNetworkState = (): Promise<NetworkInfo> => {
	return invoke<NetworkInfo>('plugin:network-manager|get_network_state');
};

export const listWifiNetworks = (options?: {
	forceRefresh?: boolean;
	ttlMs?: number;
}): Promise<NetworkInfo[]> => {
	return invoke<NetworkInfo[]>('plugin:network-manager|list_wifi_networks', {
		force_refresh: options?.forceRefresh,
		ttl_ms: options?.ttlMs,
	});
};

export const connectToWifi = (config: WiFiConnectionConfig): Promise<void> => {
	return invoke<void>('plugin:network-manager|connect_to_wifi', { config });
};

export const isWirelessAvailable = (): Promise<boolean> => {
	return invoke<boolean>('plugin:network-manager|is_wireless_available');
};

export const getWirelessEnabled = (): Promise<boolean> => {
	return invoke<boolean>('plugin:network-manager|get_wireless_enabled');
};

export const setWirelessEnabled = (enabled: boolean): Promise<boolean> => {
	return invoke<boolean>('plugin:network-manager|set_wireless_enabled', { enabled });
};

export const getVpnStatus = (): Promise<VpnStatus> => {
	return invoke<VpnStatus>('plugin:network-manager|get_vpn_status');
};

export const toggleNetworkApplet = async () => {
	await invoke('toggle_network_applet');
};
