<template>
  <div class="flex flex-col h-full p-2">
    <!-- Header -->
    <div class="flex justify-between items-center mb-4">
      <h2 class="text-xl font-semibold text-tx-main">{{ t('components.NetworkControlArea.title') }}</h2>
      <button
        v-if="!hideX"
        @click="closeApplet"
        class="p-2 rounded-corner border border-ui-border bg-ui-surface/50 hover:bg-ui-surface transition-colors"
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

    <!-- Wi-Fi, with the live traffic riding along on the same row: it used to
         own a block of its own, and that block was most of the space the list
         of networks was missing. -->
    <div
      class="flex items-center gap-3 mb-4 p-3 rounded-corner border border-ui-border bg-ui-surface/45"
    >
      <template v-if="wifiAvailable">
        <div class="p-2 rounded-full bg-primary/10">
          <svg
            class="w-5 h-5 text-primary"
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
        <div class="min-w-0">
          <h3 class="font-medium text-tx-main">Wi-Fi</h3>
          <p class="text-sm text-tx-muted truncate">{{ wifiStatus }}</p>
        </div>
      </template>

      <span v-else class="text-sm text-tx-muted">{{
        t('components.NetworkControlArea.wifiUnavailable')
      }}</span>

      <div class="flex-1"></div>

      <div
        class="flex items-center gap-3 text-xs text-tx-muted"
        :title="t('components.NetworkControlArea.realtimeTraffic')"
      >
        <span class="truncate max-w-28">{{ statsInterfaceLabel }}</span>
        <span
          class="flex items-center gap-1 tabular-nums"
          :title="t('components.NetworkControlArea.download')"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m0 0l-6-6m6 6l6-6"></path>
          </svg>
          {{ downloadSpeedLabel }}
        </span>
        <span
          class="flex items-center gap-1 tabular-nums"
          :title="t('components.NetworkControlArea.upload')"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 20V4m0 0l-6 6m6-6l6 6"></path>
          </svg>
          {{ uploadSpeedLabel }}
        </span>
      </div>

      <SwitchToggle
        v-if="wifiAvailable"
        :is-on="wifiEnabled"
        @toggle="toggleWifi"
      />
    </div>

    <div v-if="wifiAvailable && wifiEnabled" class="flex-1 flex flex-col min-h-0">
      <h3 class="text-sm font-medium text-tx-main mb-3">
        {{ t('components.NetworkControlArea.availableNetworks') }}
      </h3>

      <div v-if="loading" class="flex-1 flex items-center justify-center py-8">
        <div
          class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"
        ></div>
      </div>

      <div v-else class="flex-1 min-h-0 space-y-2 overflow-y-auto pr-1">
        <NetworkWiFiCard
          v-for="network in availableNetworks"
          :key="network.ssid"
          v-bind="network"
        />
      </div>

      <button
        @click="refreshNetworks"
        class="w-full mt-4 p-2 rounded-corner border border-ui-border bg-ui-surface/50 hover:bg-ui-surface transition-colors text-sm text-tx-main"
      >
        {{ t('components.NetworkControlArea.refresh') }}
      </button>
    </div>

    <!-- The two wired states, side by side: one line each is all they say. -->
    <div class="mt-4 grid grid-cols-2 gap-3">
      <div
        class="flex items-center gap-3 p-3 rounded-corner border border-ui-border bg-ui-surface/45"
      >
        <div class="p-2 rounded-full bg-primary/10">
          <svg
            class="w-5 h-5 text-primary"
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
        <div class="min-w-0">
          <h3 class="font-medium text-tx-main">Ethernet</h3>
          <p class="text-sm text-tx-muted truncate">{{ ethernetStatus }}</p>
        </div>
      </div>

      <div
        class="flex items-center gap-3 p-3 rounded-corner border border-ui-border bg-ui-surface/45"
      >
        <div class="p-2 rounded-full bg-primary/10">
          <svg
            class="w-5 h-5"
            :class="vpnConnected ? 'text-status-success' : 'text-tx-muted'"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 3l7 3v5c0 4.418-2.99 8.166-7 9-4.01-.834-7-4.582-7-9V6l7-3z"
            ></path>
          </svg>
        </div>
        <div class="min-w-0">
          <h3 class="font-medium text-tx-main">VPN</h3>
          <p class="text-sm text-tx-muted truncate">{{ vpnLabel }}</p>
          <!-- Por dónde sale y con qué dirección: lo que alguien mira cuando
               quiere saber si está entrando a la red de la oficina. -->
          <p v-if="vpnDetail" class="text-xs text-tx-muted truncate">{{ vpnDetail }}</p>
        </div>
      </div>

      <!-- Twingate no habla por NetworkManager, así que lo que la tarjeta de
           arriba puede decir es «hay un túnel». Lo que hace falta saber —a qué
           se puede entrar y qué hay que autorizar— lo sabe su cliente. -->
      <TwingateArea />
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import TwingateArea from '@/components/areas/network/TwingateArea.vue';
import NetworkWiFiCard from '@/components/cards/NetworkWiFiCard.vue';
import SwitchToggle from '@/components/forms/SwitchToggle.vue';
import {
	getCurrentNetworkState,
	getNetworkStats,
	getVpnStatus,
	getWirelessEnabled,
	isWirelessAvailable,
	listWifiNetworks,
	type NetworkInfo,
	type NetworkStats,
	setWirelessEnabled,
	type VpnStatus,
} from '@/services/network.service';
import { toggleNetworkApplet } from '@/services/window.service';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const wifiEnabled = ref(true);
const wifiAvailable = ref(true);
const loading = ref(false);
const availableNetworks = ref<NetworkInfo[]>([]);
const networkStats = ref<NetworkStats | null>(null);
const vpnStatus = ref<VpnStatus | null>(null);
let statsPollInterval: ReturnType<typeof setInterval> | undefined;
const wifiStatus = ref(t('components.NetworkControlArea.checking'));
const ethernetStatus = ref(t('components.NetworkControlArea.checking'));

const vpnConnected = computed(() => vpnStatus.value?.state === 'connected');
const vpnDetail = computed(() => {
	if (!vpnConnected.value) return '';
	return [vpnStatus.value?.interface, vpnStatus.value?.ip_address].filter(Boolean).join(' · ');
});
const vpnLabel = computed(() => {
	if (!vpnConnected.value) return t('components.NetworkControlArea.vpnInactive');
	return vpnStatus.value?.active_profile_name
		? t('components.NetworkControlArea.vpnConnectedTo').replace(
				'{0}',
				vpnStatus.value.active_profile_name
			)
		: t('components.NetworkControlArea.vpnActive');
});

const formatBytesPerSecond = (value?: number) => {
	const safe = Math.max(0, value ?? 0);
	if (safe < 1024) return `${safe.toFixed(0)} B/s`;
	if (safe < 1024 * 1024) return `${(safe / 1024).toFixed(1)} KB/s`;
	if (safe < 1024 * 1024 * 1024) return `${(safe / (1024 * 1024)).toFixed(1)} MB/s`;
	return `${(safe / (1024 * 1024 * 1024)).toFixed(2)} GB/s`;
};

const downloadSpeedLabel = computed(() => formatBytesPerSecond(networkStats.value?.download_speed));
const uploadSpeedLabel = computed(() => formatBytesPerSecond(networkStats.value?.upload_speed));
const statsInterfaceLabel = computed(
	() => networkStats.value?.interface || t('components.NetworkControlArea.noInterface')
);

defineProps({
	hideX: {
		type: Boolean,
		default: false,
	},
});

const checkWirelessStatus = async () => {
	try {
		const available = await isWirelessAvailable();
		wifiAvailable.value = available;

		if (available) {
			const enabled = await getWirelessEnabled();
			wifiEnabled.value = enabled;
			wifiStatus.value = enabled
				? t('components.NetworkControlArea.wifiOn')
				: t('components.NetworkControlArea.wifiOff');

			if (enabled) {
				await refreshNetworks();
			}
		} else {
			wifiStatus.value = t('components.NetworkControlArea.wifiNotAvailable');
			wifiEnabled.value = false;
		}
	} catch (e) {
		logError('Error verificando estado wireless:', e);
	}
};

const toggleWifi = async () => {
	if (!wifiAvailable.value) return;

	try {
		const newState = !wifiEnabled.value;
		await setWirelessEnabled(newState);

		wifiEnabled.value = newState;
		wifiStatus.value = newState
			? t('components.NetworkControlArea.wifiOn')
			: t('components.NetworkControlArea.wifiOff');

		if (wifiEnabled.value) {
			await refreshNetworks();
		} else {
			availableNetworks.value = [];
		}
	} catch (error) {
		logError('Error toggling WiFi:', error);
	}
};

const refreshVpnStatus = async () => {
	try {
		vpnStatus.value = await getVpnStatus();
	} catch (error) {
		vpnStatus.value = null;
		logError('Error fetching VPN status:', error);
	}
};

const refreshNetworkStats = async () => {
	try {
		networkStats.value = await getNetworkStats();
	} catch (error) {
		logError('Error fetching network stats:', error);
	}
};

const updateEthernetStatus = (state: NetworkInfo | null) => {
	if (!state) {
		ethernetStatus.value = t('components.NetworkControlArea.ethernetUnknown');
		return;
	}

	const isEthernet = state.connection_type?.toLowerCase() === 'ethernet';
	if (isEthernet && state.is_connected) {
		ethernetStatus.value = t('components.NetworkControlArea.ethernetConnected');
		return;
	}

	if (isEthernet && !state.is_connected) {
		ethernetStatus.value = t('components.NetworkControlArea.ethernetDisconnected');
		return;
	}

	ethernetStatus.value = t('components.NetworkControlArea.ethernetNoLink');
};

const refreshEthernetStatus = async () => {
	try {
		const state = await getCurrentNetworkState();
		updateEthernetStatus(state);
	} catch (error) {
		logError('Error fetching ethernet status:', error);
		ethernetStatus.value = t('components.NetworkControlArea.ethernetUnknown');
	}
};

const refreshNetworks = async () => {
	if (!wifiEnabled.value || !wifiAvailable.value) return;
	loading.value = true;
	try {
		availableNetworks.value = await listWifiNetworks();
	} catch (error) {
		logError('Error refreshing networks:', error);
	} finally {
		loading.value = false;
	}
};

const closeApplet = async () => {
	try {
		await toggleNetworkApplet();
	} catch (error) {
		logError('Error closing applet:', error);
	}
};

onMounted(async () => {
	await checkWirelessStatus();
	await refreshEthernetStatus();
	await refreshVpnStatus();
	await refreshNetworkStats();
	// Only poll while the window is actually on screen. The control center is
	// hidden rather than destroyed now, so an unconditional timer would keep
	// querying NetworkManager every two seconds for a panel nobody is looking
	// at, for the whole session.
	const startPolling = () => {
		if (statsPollInterval !== undefined) return;
		statsPollInterval = setInterval(() => {
			void refreshNetworkStats();
		}, 2000);
	};

	const stopPolling = () => {
		if (statsPollInterval === undefined) return;
		clearInterval(statsPollInterval);
		statsPollInterval = undefined;
	};

	document.addEventListener('visibilitychange', () => {
		if (document.hidden) stopPolling();
		else {
			void refreshNetworkStats();
			startPolling();
		}
	});

	startPolling();
});

onUnmounted(() => {
	clearInterval(statsPollInterval);
});

useSharedEvent<any>('network-changed', async () => {
	await checkWirelessStatus();
	await refreshEthernetStatus();
	await refreshNetworkStats();
});

useSharedEvent('vpn-changed', refreshVpnStatus);
</script>
