import { describe, expect, test, beforeAll, afterAll, beforeEach } from 'bun:test';

const tauriCalls: Array<{ cmd: string; args: unknown }> = [];

beforeAll(() => {
	(globalThis as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
		metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
		transformCallback: () => 0,
		convertFileSrc: (path: string) => path,
		invoke: async (cmd: string, args: unknown) => {
			tauriCalls.push({ cmd, args });
			if (cmd === 'plugin:network-manager|get_saved_wifi_networks') return [];
			if (cmd === 'plugin:network-manager|delete_wifi_connection') return undefined;
			if (cmd === 'plugin:network-manager|connect_to_wifi') return undefined;
			return '';
		},
	};
});

afterAll(() => {
	delete (globalThis as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
	tauriCalls.length = 0;
});

import {
	getSavedWifiNetworks,
	deleteWifiConnection,
	connectToWifi,
	type NetworkInfo,
} from '@/services/network.service';

describe('network.service', () => {
	beforeEach(() => {
		tauriCalls.length = 0;
	});

	describe('getSavedWifiNetworks', () => {
		test('invoca el comando correcto y devuelve la lista', async () => {
			const redes: NetworkInfo[] = [
				{
					name: 'MiWiFi',
					ssid: 'MiWiFi',
					connection_type: 'wifi',
					icon: 'network-wireless-signal-good-symbolic',
					ip_address: '0.0.0.0',
					mac_address: '00:11:22:33:44:55',
					signal_strength: 80,
					security_type: 'wpa2-psk',
					is_connected: false,
				},
			];

			const result = await getSavedWifiNetworks();

			expect(tauriCalls).toContainEqual({
				cmd: 'plugin:network-manager|get_saved_wifi_networks',
				args: {},
			});
			expect(result).toEqual([]);
		});
	});

	describe('deleteWifiConnection', () => {
		test('invoca el comando correcto con el SSID', async () => {
			await deleteWifiConnection('MiWiFi');

			expect(tauriCalls).toContainEqual({
				cmd: 'plugin:network-manager|delete_wifi_connection',
				args: { ssid: 'MiWiFi' },
			});
		});
	});

	describe('connectToWifi', () => {
		test('invoca el comando correcto con la config completa', async () => {
			await connectToWifi({
				ssid: 'MiWiFi',
				password: 'secreto123',
				security_type: 'wpa2-psk',
			});

			expect(tauriCalls).toContainEqual({
				cmd: 'plugin:network-manager|connect_to_wifi',
				args: {
					config: {
						ssid: 'MiWiFi',
						password: 'secreto123',
						security_type: 'wpa2-psk',
					},
				},
			});
		});

		test('invoca el comando con config mínima (perfil guardado, sin password)', async () => {
			await connectToWifi({
				ssid: 'MiWiFi',
				security_type: 'wpa2-psk',
			});

			expect(tauriCalls).toContainEqual({
				cmd: 'plugin:network-manager|connect_to_wifi',
				args: {
					config: {
						ssid: 'MiWiFi',
						security_type: 'wpa2-psk',
					},
				},
			});
		});
	});
});