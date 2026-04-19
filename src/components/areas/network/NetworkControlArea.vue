<template>
  <div class="flex flex-col h-full p-2">
    <!-- Header -->
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-xl font-semibold text-vsk-text">Redes</h2>
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

    <!-- WiFi Toggle -->
    <div
      v-if="wifiAvailable"
      class="flex items-center justify-between mb-4 p-3 rounded-corner border border-ui-border bg-ui-surface/45"
    >
      <div class="flex items-center gap-3">
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
        <div>
          <h3 class="font-medium text-vsk-text">Wi-Fi</h3>
          <p class="text-sm text-vsk-text/70">{{ wifiStatus }}</p>
        </div>
      </div>
      <SwitchToggle
        :is-on="wifiEnabled"
        @toggle="toggleWifi"
      />
    </div>

    <div
      v-else
      class="mb-4 p-3 rounded-corner border border-ui-border bg-ui-surface/45 flex items-center justify-center"
    >
      <span class="text-sm text-tx-muted"
        >Wi-Fi no disponible en este dispositivo</span
      >
    </div>

    <div v-if="wifiAvailable && wifiEnabled" class="flex-1 overflow-hidden">
      <h3 class="text-sm font-medium text-vsk-text/80 mb-3">
        Redes disponibles
      </h3>

      <div v-if="loading" class="flex items-center justify-center py-8">
        <div
          class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"
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
        class="w-full mt-4 p-2 rounded-corner border border-ui-border bg-ui-surface/50 hover:bg-ui-surface transition-colors text-sm text-vsk-text"
      >
        Actualizar redes
      </button>
    </div>

	<div class="mt-4">
		<div class="flex items-center gap-3 p-3 rounded-corner border border-ui-border bg-ui-surface/45">
			<div
				class="w-2.5 h-2.5 rounded-full"
				:class="vpnConnected ? 'bg-status-success animate-pulse' : 'bg-status-danger'"
			></div>
			<div class="flex-1 min-w-0">
				<h3 class="font-medium text-vsk-text">VPN</h3>
				<p class="text-sm text-vsk-text/70 truncate">{{ vpnLabel }}</p>
			</div>
		</div>
	</div>

  <div class="mt-4">
    <div class="rounded-corner border border-ui-border bg-ui-surface/45 p-3">
      <div class="flex items-center justify-between">
        <h3 class="font-medium text-vsk-text">Tráfico en tiempo real</h3>
        <span class="text-xs text-vsk-text/60 truncate max-w-32 text-right">{{ statsInterfaceLabel }}</span>
      </div>
      <div class="mt-3 grid grid-cols-2 gap-3">
        <div class="rounded-corner bg-ui-surface/60 border border-ui-border p-2">
          <p class="text-xs text-vsk-text/70">Descarga</p>
          <p class="text-sm font-semibold text-vsk-text">{{ downloadSpeedLabel }}</p>
        </div>
        <div class="rounded-corner bg-ui-surface/60 border border-ui-border p-2">
          <p class="text-xs text-vsk-text/70">Subida</p>
          <p class="text-sm font-semibold text-vsk-text">{{ uploadSpeedLabel }}</p>
        </div>
      </div>
    </div>
  </div>

    <div class="mt-6 pt-4 border-t border-primary/70">
      <div
        class="flex items-center gap-3 p-3 bg-ui-surface/45 rounded-corner border border-ui-border"
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
        <div>
          <h3 class="font-medium text-vsk-text">Ethernet</h3>
          <p class="text-sm text-vsk-text/70">{{ ethernetStatus }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { listen } from '@tauri-apps/api/event';
import { computed, onMounted, onUnmounted, type Ref, ref } from 'vue';
import NetworkWiFiCard from '@/components/cards/NetworkWiFiCard.vue';
import SwitchToggle from '@/components/forms/SwitchToggle.vue';
import {
  getCurrentNetworkState,
	getNetworkStats,
  getVpnStatus,
  listWifiNetworks,
	getWirelessEnabled,
	isWirelessAvailable,
	setWirelessEnabled,
  type NetworkStats,
  type NetworkInfo,
  type VpnStatus,
} from '@/services/network.service';
import { toggleNetworkApplet } from '@/services/window.service';
import { logError } from '@/utils/logger';

const wifiEnabled: Ref<boolean> = ref(true);
const wifiAvailable: Ref<boolean> = ref(true);
const loading: Ref<boolean> = ref(false);
const availableNetworks: Ref<NetworkInfo[]> = ref([]);
const networkStats: Ref<NetworkStats | null> = ref(null);
const vpnStatus: Ref<VpnStatus | null> = ref(null);
const wifiStatus: Ref<string> = ref('Checking...');
const ethernetStatus: Ref<string> = ref('Checking...');
let unlisten: (() => void) | null = null;
let unlistenVpn: (() => void) | null = null;
let statsInterval: ReturnType<typeof setInterval> | null = null;

const vpnConnected = computed(() => vpnStatus.value?.state === 'connected');
const vpnLabel = computed(() => {
  if (!vpnConnected.value) return 'Sin conexión VPN activa';
  return vpnStatus.value?.active_profile_name
    ? `Conectada: ${vpnStatus.value.active_profile_name}`
    : 'Conexión VPN activa';
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
const statsInterfaceLabel = computed(() => networkStats.value?.interface || 'Sin interfaz');

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
      wifiStatus.value = enabled ? 'Activado' : 'Desactivado';

			if (enabled) {
				await refreshNetworks();
			}
		} else {
      wifiStatus.value = 'No disponible';
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
    wifiStatus.value = newState ? 'Activado' : 'Desactivado';

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
    ethernetStatus.value = 'Desconocido';
		return;
	}

	const isEthernet = state.connection_type?.toLowerCase() === 'ethernet';
	if (isEthernet && state.is_connected) {
    ethernetStatus.value = 'Conectado';
		return;
	}

	if (isEthernet && !state.is_connected) {
    ethernetStatus.value = 'Desconectado';
		return;
	}

  ethernetStatus.value = 'Sin enlace';
};

const refreshEthernetStatus = async () => {
	try {
		const state = await getCurrentNetworkState();
		updateEthernetStatus(state);
	} catch (error) {
		logError('Error fetching ethernet status:', error);
    ethernetStatus.value = 'Desconocido';
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
	unlisten = await listen<any>('network-changed', async () => {
		await checkWirelessStatus();
		await refreshEthernetStatus();
    await refreshNetworkStats();
	});
  unlistenVpn = await listen('vpn-changed', refreshVpnStatus);
  statsInterval = setInterval(() => {
    void refreshNetworkStats();
  }, 2000);
});

onUnmounted(() => {
	if (unlisten) unlisten();
  if (unlistenVpn) unlistenVpn();
  if (statsInterval) clearInterval(statsInterval);
});
</script>
