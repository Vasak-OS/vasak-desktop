<script setup lang="ts">
/**
 * El mosaico de la red (vasak-desktop#175): el cuerpo prende y apaga el Wi-Fi,
 * y la flecha abre el detalle dentro del bloque (el panel del applet de red,
 * sin su botón de cerrar). En un equipo sin radio no hay nada que alternar: el
 * cuerpo abre el detalle, donde están el cable y la VPN.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { QuickSettingsTile } from '@vasakgroup/vue-libvasak';
import { computed, onMounted } from 'vue';
import { useNetworkState } from '@/tools/composables/useNetworkState';
import { useWifiToggle } from '@/tools/composables/useWifiToggle';
import { useSharedEvent } from '@/tools/event.bus';

const emit = defineEmits<{ open: [] }>();
const { t } = useI18n();
const { networkState, networkIconName, getCurrentNetwork } = useNetworkState();
const wifi = useWifiToggle();

const status = computed(() => {
	if (wifi.available.value && !wifi.enabled.value)
		return t('components.ControlCenterTiles.disabled');
	return networkState.value.is_connected
		? String(networkState.value.ssid || networkState.value.name)
		: t('components.ControlCenterTiles.disconnected');
});

function onActivate(): void {
	if (wifi.available.value) void wifi.toggle();
	else emit('open');
}

onMounted(() => {
	void getCurrentNetwork();
	void wifi.refresh();
});

useSharedEvent('network-changed', () => {
	void wifi.refresh();
});
</script>

<template>
  <QuickSettingsTile
    :icon="networkIconName"
    :title="t('components.ControlCenterTiles.network')"
    :status="status"
    :active="wifi.available.value ? wifi.enabled.value : null"
    :loading="wifi.busy.value"
    detail
    @activate="onActivate"
    @detail="emit('open')"
  />
</template>
