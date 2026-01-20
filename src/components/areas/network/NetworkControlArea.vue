<template>
  <div class="flex flex-col h-full p-2">
    <!-- Header -->
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-xl font-semibold text-vsk-text">Network Settings</h2>
      <button
        v-if="!hideX"
        @click="closeApplet"
        class="p-2 rounded-vsk hover:bg-vsk-primary/10 transition-colors"
      >
        <svg
          class="w-5 h-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M6 18L18 6M6 6l12 12"
          ></path>
        </svg>
      </button>
    </div>

    <!-- WiFi Toggle -->
    <div
      v-if="wifiAvailable"
      class="flex items-center justify-between mb-4 p-3 rounded-vsk border border-vsk-primary/70 background"
    >
      <div class="flex items-center gap-3">
        <div class="p-2 rounded-full bg-vsk-primary/10">
          <svg
            class="w-5 h-5 text-vsk-primary"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0"
            ></path>
          </svg>
        </div>
        <div>
          <h3 class="font-medium text-vsk-text">WiFi</h3>
          <p class="text-sm text-vsk-text/70">{{ wifiStatus }}</p>
        </div>
      </div>
      <button
        @click="toggleWifi"
        :class="[
          'relative inline-flex h-6 w-11 items-center rounded-full transition-colors',
          wifiEnabled ? 'bg-green-500' : 'bg-vsk-primary/30',
        ]"
      >
        <span
          :class="[
            'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
            wifiEnabled ? 'translate-x-6' : 'translate-x-1',
          ]"
        />
      </button>
    </div>

    <div
      v-else
      class="mb-4 p-3 rounded-vsk border border-gray-200 dark:border-gray-700 bg-gray-50/50 dark:bg-gray-800/50 flex items-center justify-center"
    >
      <span class="text-sm text-gray-500"
        >Wireless hardware not cached or unavailable</span
      >
    </div>

    <div v-if="wifiAvailable && wifiEnabled" class="flex-1 overflow-hidden">
      <h3 class="text-sm font-medium text-vsk-text/80 mb-3">
        Available Networks
      </h3>

      <div v-if="loading" class="flex items-center justify-center py-8">
        <div
          class="animate-spin rounded-full h-8 w-8 border-b-2 border-vsk-primary"
        ></div>
      </div>

      <div v-else class="space-y-2 overflow-y-auto max-h-96">
        <NetworkWiFiCard
          v-for="network in availableNetworks"
          :key="network.ssid"
          v-bind="network"
        />
      </div>

      <button
        @click="refreshNetworks"
        class="w-full mt-4 p-2 rounded-vsk border border-vsk-primary/70 hover:bg-vsk-primary/5 transition-colors text-sm text-vsk-text"
      >
        Refresh Networks
      </button>
    </div>

    <div class="mt-6 pt-4 border-t border-vsk-primary/70">
      <div
        class="flex items-center gap-3 p-3 background rounded-vsk border border-vsk-primary/70"
      >
        <div class="p-2 rounded-full bg-vsk-primary/10">
          <svg
            class="w-5 h-5 text-vsk-primary"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2"
            ></path>
          </svg>
        </div>
        <div>
          <h3 class="font-medium text-vsk-text">Ethernet</h3>
          <p class="text-sm text-vsk-text/70">{{ ethernetStatus }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
	listWifiNetworks,
	NetworkInfo,
	getCurrentNetworkState,
} from '@vasakgroup/plugin-network-manager';
import NetworkWiFiCard from '@/components/cards/NetworkWiFiCard.vue';

const wifiEnabled: Ref<boolean> = ref(true);
const wifiAvailable: Ref<boolean> = ref(true);
const loading: Ref<boolean> = ref(false);
const availableNetworks: Ref<NetworkInfo[]> = ref([]);
const wifiStatus: Ref<string> = ref('Checking...');
const ethernetStatus: Ref<string> = ref('Checking...');
let unlisten: (() => void) | null = null;

defineProps({
	hideX: {
		type: Boolean,
		default: false,
	},
});

const checkWirelessStatus = async () => {
	try {
		const available = await invoke(
			'plugin:network-manager|is_wireless_available'
		);
		wifiAvailable.value = available as boolean;

		if (available) {
			const enabled = await invoke(
				'plugin:network-manager|get_wireless_enabled'
			);
			wifiEnabled.value = enabled as boolean;
			wifiStatus.value = enabled ? 'On' : 'Off';

			if (enabled) {
				await refreshNetworks();
			}
		} else {
			wifiStatus.value = 'Hardware unavailable';
			wifiEnabled.value = false;
		}
	} catch (e) {
		console.error('Error checking wireless status:', e);
	}
};

const toggleWifi = async () => {
	if (!wifiAvailable.value) return;

	try {
		const newState = !wifiEnabled.value;
		await invoke('plugin:network-manager|set_wireless_enabled', {
			enabled: newState,
		});

		wifiEnabled.value = newState;
		wifiStatus.value = newState ? 'On' : 'Off';

		if (wifiEnabled.value) {
			await refreshNetworks();
		} else {
			availableNetworks.value = [];
		}
	} catch (error) {
		console.error('Error toggling WiFi:', error);
	}
};

const updateEthernetStatus = (state: NetworkInfo | null) => {
	if (!state) {
		ethernetStatus.value = 'Unknown';
		return;
	}

	const isEthernet = state.connection_type?.toLowerCase() === 'ethernet';
	if (isEthernet && state.is_connected) {
		ethernetStatus.value = 'Connected';
		return;
	}

	if (isEthernet && !state.is_connected) {
		ethernetStatus.value = 'Disconnected';
		return;
	}

	ethernetStatus.value = 'Not connected';
};

const refreshEthernetStatus = async () => {
	try {
		const state = await getCurrentNetworkState();
		updateEthernetStatus(state);
	} catch (error) {
		console.error('Error fetching ethernet status:', error);
		ethernetStatus.value = 'Unknown';
	}
};

const refreshNetworks = async () => {
	if (!wifiEnabled.value || !wifiAvailable.value) return;
	loading.value = true;
	try {
		availableNetworks.value = await listWifiNetworks();
	} catch (error) {
		console.error('Error refreshing networks:', error);
	} finally {
		loading.value = false;
	}
};

const closeApplet = async () => {
	try {
		await invoke('toggle_network_applet');
	} catch (error) {
		console.error('Error closing applet:', error);
	}
};

onMounted(async () => {
	await checkWirelessStatus();
	await refreshEthernetStatus();
	unlisten = await listen<any>('network-changed', async () => {
		await checkWirelessStatus();
		await refreshEthernetStatus();
	});
});

onUnmounted(() => {
	if (unlisten) unlisten();
});
</script>
