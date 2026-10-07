<script setup lang="ts">
/**
 * El mosaico de la red (vasak-desktop#175): el cuerpo prende y apaga el Wi-Fi,
 * y la flecha abre el detalle dentro del bloque (el panel del applet de red,
 * sin su botón de cerrar). En un equipo sin radio no hay nada que alternar: el
 * cuerpo abre el detalle, donde están el cable y la VPN.
 *
 * Con el Wi-Fi bloqueado por rfkill —el modo avión, la tecla del teclado— se
 * ve apagado en el acto, sin esperar a que NetworkManager se entere
 * (vasak-desktop#180).
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { QuickSettingsTile } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, watch } from 'vue';
import { useAirplaneMode } from '@/tools/composables/useAirplaneMode';
import { useNetworkState } from '@/tools/composables/useNetworkState';
import { useWifiToggle } from '@/tools/composables/useWifiToggle';
import { useSharedEvent } from '@/tools/event.bus';

const emit = defineEmits<{ open: [] }>();
const { t } = useI18n();
const { networkState, networkIconName, getCurrentNetwork } = useNetworkState();
const wifi = useWifiToggle();
const { wlanBlocked, unblockRadios } = useAirplaneMode();
const wifiOn = computed(() => wifi.enabled.value && !wlanBlocked.value);

const status = computed(() => {
	if (wifi.available.value && !wifiOn.value) return t('components.ControlCenterTiles.disabled');
	return networkState.value.is_connected
		? String(networkState.value.ssid || networkState.value.name)
		: t('components.ControlCenterTiles.disconnected');
});

async function unblockWifi(): Promise<void> {
	try {
		await unblockRadios('wlan');
	} catch (error) {
		console.error('[wifi] no se pudo desbloquear la radio:', error);
	}
}

function onActivate(): void {
	// Bloqueada por rfkill, tocarla la desbloquea: NetworkManager la vuelve a
	// prender sola, y el evento del modo avión hace releer el estado.
	if (wlanBlocked.value) void unblockWifi();
	else if (wifi.available.value) void wifi.toggle();
	else emit('open');
}

onMounted(() => {
	void getCurrentNetwork();
	void wifi.refresh();
});

useSharedEvent('network-changed', () => {
	void wifi.refresh();
});

// Al desbloquearse, NetworkManager vuelve a prender la radio: se relee.
watch(wlanBlocked, () => {
	void wifi.refresh();
});
</script>

<template>
  <QuickSettingsTile
    :icon="networkIconName"
    :title="t('components.ControlCenterTiles.network')"
    :status="status"
    :active="wifi.available.value ? wifiOn : null"
    :loading="wifi.busy.value"
    detail
    @activate="onActivate"
    @detail="emit('open')"
  />
</template>
