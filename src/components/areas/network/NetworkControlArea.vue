<template>
  <!-- Las piezas son las de la librería: el botón de cerrar y el de actualizar
       son `ActionButton`, cada bloque es una tarjeta (`ListCard`), la rueda de
       carga es `LoadingState` y los iconos salen del tema por `ThemeIcon`.
       Antes cada uno era un dibujo a mano —seis SVG en línea— que no seguía
       al tema de iconos que eligió la persona.

       Es un contenedor: con menos de 24rem las dos tarjetas de cable se
       apilan en vez de cortar lo que dicen. -->
  <div class="@container flex flex-col h-full p-2">
    <!-- Header -->
    <div class="flex justify-between items-center gap-3 mb-4">
      <h2 class="min-w-0 break-words text-xl font-semibold text-tx-main">{{ t('components.NetworkControlArea.title') }}</h2>
      <ActionButton
        v-if="!hideX"
        label=""
        icon="window-close"
        :icon-alt="t('common.close')"
        :title="t('common.close')"
        variant="secondary"
        @click="closeApplet"
      />
    </div>

    <!-- Wi-Fi, with the live traffic riding along on the same row: it used to
         own a block of its own, and that block was most of the space the list
         of networks was missing. -->
    <ListCard custom-class="mb-4 flex-wrap">
      <template v-if="wifiAvailable">
        <span class="flex shrink-0 items-center justify-center rounded-corner-full bg-ui-selected-accent p-2">
          <ThemeIcon name="network-wireless" type="symbol" :size="20" alt="" />
        </span>
        <div class="min-w-0">
          <h3 class="font-medium text-tx-main">Wi-Fi</h3>
          <p class="text-label-m text-tx-muted truncate" :title="wifiStatus">{{ wifiStatus }}</p>
        </div>
      </template>

      <span v-else class="text-label-m text-tx-muted">{{
        t('components.NetworkControlArea.wifiUnavailable')
      }}</span>

      <div class="flex-1"></div>

      <div
        class="flex min-w-0 flex-wrap items-center gap-3 text-label-xs text-tx-muted"
        :title="t('components.NetworkControlArea.realtimeTraffic')"
      >
        <span class="truncate max-w-28" :title="statsInterfaceLabel">{{ statsInterfaceLabel }}</span>
        <span
          class="flex items-center gap-1 tabular-nums"
          :title="t('components.NetworkControlArea.download')"
        >
          <ThemeIcon name="go-down" type="symbol" :size="14" :alt="t('components.NetworkControlArea.download')" />
          {{ downloadSpeedLabel }}
        </span>
        <span
          class="flex items-center gap-1 tabular-nums"
          :title="t('components.NetworkControlArea.upload')"
        >
          <ThemeIcon name="go-up" type="symbol" :size="14" :alt="t('components.NetworkControlArea.upload')" />
          {{ uploadSpeedLabel }}
        </span>
      </div>

      <SwitchToggle :label="t('components.NetworkControlArea.wifiToggle')"
        v-if="wifiAvailable"
        :model-value="wifiEnabled"
        @update:model-value="toggleWifi"
      />
    </ListCard>

    <div v-if="wifiAvailable && wifiEnabled" class="flex-1 flex flex-col min-h-0">
      <h3 class="text-label-m font-medium text-tx-main mb-3">
        {{ t('components.NetworkControlArea.availableNetworks') }}
      </h3>

      <div v-if="loading" class="flex-1 flex items-center justify-center">
        <LoadingState :label="t('common.loading')" />
      </div>

      <div v-else class="flex-1 min-h-0 space-y-2 overflow-y-auto pr-1">
        <NetworkWiFiCard
          v-for="network in availableNetworks"
          :key="network.ssid"
          v-bind="network"
        />
      </div>

      <ActionButton
        :label="t('components.NetworkControlArea.refresh')"
        variant="secondary"
        full-width
        class="mt-4"
        @click="refreshNetworks"
      />
    </div>

    <!-- The two wired states, side by side: one line each is all they say. -->
    <div class="mt-4 grid grid-cols-1 gap-3 @sm:grid-cols-2">
      <ListCard>
        <div class="flex min-w-0 flex-1 items-center gap-3">
          <span class="flex shrink-0 items-center justify-center rounded-corner-full bg-ui-selected-accent p-2">
            <ThemeIcon name="network-wired" type="symbol" :size="20" alt="" />
          </span>
          <div class="min-w-0">
            <h3 class="font-medium text-tx-main">Ethernet</h3>
            <p class="text-label-m text-tx-muted truncate" :title="ethernetStatus">{{ ethernetStatus }}</p>
          </div>
        </div>
      </ListCard>

      <ListCard>
        <div class="flex min-w-0 flex-1 items-center gap-3">
        <span class="flex shrink-0 items-center justify-center rounded-corner-full bg-ui-selected-accent p-2">
          <ThemeIcon
            name="network-vpn"
            type="symbol"
            :size="20"
            alt=""
            :class="vpnConnected ? 'text-status-success' : 'text-tx-muted'"
          />
        </span>
        <div class="min-w-0">
          <h3 class="font-medium text-tx-main">VPN</h3>
          <p class="text-label-m text-tx-muted truncate" :title="vpnLabel">{{ vpnLabel }}</p>
          <!-- Por dónde sale y con qué dirección: lo que alguien mira cuando
               quiere saber si está entrando a la red de la oficina. -->
          <p v-if="vpnDetail" class="text-label-xs text-tx-muted truncate" :title="vpnDetail">{{ vpnDetail }}</p>
        </div>
        </div>
      </ListCard>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	ListCard,
	LoadingState,
	SwitchToggle,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import NetworkWiFiCard from '@/components/cards/NetworkWiFiCard.vue';
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
import { dismissApplet } from '@/services/window.service';
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
		await dismissApplet('network');
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
