<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { PanelPill, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import {
	getCurrentNetworkState,
	getVpnStatus,
	type NetworkInfo,
	type VpnStatus,
} from '@/services/network.service';
import { toggleApplet } from '@/services/window.service';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { usePanelDensity } from '@/tools/composables/usePanelDensity';
import { useSharedEvent } from '@/tools/event.bus';
import { showsNames } from '@/tools/panel-density';
import { logError } from '@/utils/logger';

const { t } = useI18n();
const { vertical } = usePanelConfig();
const density = usePanelDensity();

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
const networkIconName = computed(() => networkState.value.icon);

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

/**
 * Lo que dice la píldora: el nombre de la red, como en el video de referencia
 * (vasak-desktop#151). Por cable, el de la conexión. Desconectada, nada: el
 * icono ya lo dice, y la píldora queda redonda.
 */
const networkLabel = computed(() => {
	if (vertical.value || !showsNames(density.value) || !networkState.value.is_connected) return '';
	const { ssid, name } = networkState.value;
	const known = (value: string) => Boolean(value) && value !== 'Unknown';
	return known(ssid) ? ssid : known(name) ? name : '';
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

const button = ref<unknown>(null);
const { isOpen } = useOpenApplet('network');

const toggleNetworkApplet = async () => {
	try {
		await toggleApplet('network', button.value);
	} catch (error) {
		logError('Error toggling network applet:', error);
	}
};
</script>

<template>
  <!-- La píldora de la red (vasak-desktop#151): rellena en el primario
       mientras hay conexión, con el nombre de la red; translúcida y redonda
       sin ella. Abre la vista de la red, colgada de acá. La VPN puesta suma
       su candado adentro. -->
  <PanelPill
    ref="button"
    :icon="networkIconName"
    icon-type="icon"
    :label="networkLabel"
    :active="networkState.is_connected"
    :expanded="isOpen"
    :title="networkAlt"
    :accessible-label="networkAlt"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    class="max-w-48"
    data-network-pill
    @click="toggleNetworkApplet"
  >
    <ThemeIcon
      v-if="vpnConnected"
      name="network-vpn-symbolic"
      type="symbol"
      :size="14"
      alt=""
      class="shrink-0"
      data-vpn-mark
    />
  </PanelPill>
</template>
