<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { getConnectedDevices } from '@vasakgroup/plugin-bluetooth-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { PanelPill } from '@vasakgroup/vue-libvasak';
import { computed, ref, watch } from 'vue';
import { toggleApplet } from '@/services/window.service';
import { useBluetoothState } from '@/tools/bluetooth.controller';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { usePanelDensity } from '@/tools/composables/usePanelDensity';
import { showsNames } from '@/tools/panel-density';
import { logError } from '@/utils/logger';

/**
 * La píldora del Bluetooth (vasak-desktop#151): rellena en el primario con el
 * nombre del dispositivo mientras hay uno conectado —«JBL Tune 720BT»—, y
 * translúcida y redonda si no. Abre la vista del Bluetooth colgada de acá.
 */

const { t } = useI18n();
const { vertical, hasSurface } = usePanelConfig();
const density = usePanelDensity();

const { isBluetoothOn, connectedDevicesCount, defaultAdapter } = useBluetoothState({
	getIcon: async () => '',
});

const bluetoothIcon = computed(() => {
	if (!isBluetoothOn.value) return 'bluetooth-disabled-symbolic';
	return connectedDevicesCount.value > 0 ? 'bluetooth-active-symbolic' : 'bluetooth-symbolic';
});

/** El nombre del primer dispositivo conectado, o nada. */
const deviceName = ref('');

async function refreshDeviceName(): Promise<void> {
	const path = defaultAdapter.value?.path;
	if (!isBluetoothOn.value || connectedDevicesCount.value === 0 || !path) {
		deviceName.value = '';
		return;
	}
	try {
		const [device] = await getConnectedDevices(path);
		deviceName.value = device?.alias || device?.name || '';
	} catch (error) {
		deviceName.value = '';
		logError('[panel] no se pudo leer el dispositivo Bluetooth conectado:', error);
	}
}

watch([connectedDevicesCount, isBluetoothOn, () => defaultAdapter.value?.path], refreshDeviceName);

const connected = computed(() => Boolean(isBluetoothOn.value) && connectedDevicesCount.value > 0);

const description = computed(() => {
	if (!isBluetoothOn.value) return t('components.TrayIconBluetooth.statusOff');
	if (connected.value && deviceName.value) {
		return t('components.TrayIconBluetooth.connectedTo').replace('{0}', deviceName.value);
	}
	return t('components.TrayIconBluetooth.statusOn');
});

const label = computed(() =>
	vertical.value || !showsNames(density.value) || !connected.value ? '' : deviceName.value
);

const button = ref<unknown>(null);
const { isOpen } = useOpenApplet('bluetooth');

const toggleBluetooth = async (): Promise<void> => {
	try {
		await toggleApplet('bluetooth', button.value);
	} catch (error) {
		logError('Error toggling bluetooth applet:', error);
	}
};
</script>

<template>
  <PanelPill
    ref="button"
    :icon="bluetoothIcon"
    icon-type="icon"
    :label="label"
    :active="connected"
    :expanded="isOpen"
    :title="description"
    :accessible-label="description"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    :flat="hasSurface"
    class="max-w-48"
    data-bluetooth-pill
    @click="toggleBluetooth"
  />
</template>
