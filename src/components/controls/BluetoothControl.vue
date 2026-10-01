<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { toggleBluetooth } from '@vasakgroup/plugin-bluetooth-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ToggleControl, type ToggleIndicator } from '@vasakgroup/vue-libvasak';
import { computed, type Ref, ref } from 'vue';
import { useBluetoothState } from '@/tools/bluetooth.controller';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const isTogglingBluetooth: Ref<boolean> = ref(false);

const { isBluetoothOn, connectedDevicesCount } = useBluetoothState({
	getIcon: async () => '',
});

const bluetoothIcon = computed(() => {
	if (!isBluetoothOn.value) return 'bluetooth-disabled-symbolic';
	return connectedDevicesCount.value > 0 ? 'bluetooth-active-symbolic' : 'bluetooth-symbolic';
});

/**
 * El punto de la esquina y el contador de dispositivos, que son los de
 * `ToggleControl` (vue-libvasak 2.2.0) y no un punto y un número a mano encima
 * del botón —el número iba en `tx-main` sobre el primario—. El anillo de
 * `custom-class` repetía lo que ya dicen el punto y el resaltado de encendido.
 */
const indicator = computed<ToggleIndicator>(() => {
	if (!isBluetoothOn.value) return { tone: 'neutral' };
	return { tone: 'accent', pulse: connectedDevicesCount.value > 0 };
});

const toggleBT = async (): Promise<void> => {
	try {
		isTogglingBluetooth.value = true;
		await toggleBluetooth();
	} catch (error) {
		logError('[Bluetooth Control Error] Error toggling bluetooth:', error);
	} finally {
		isTogglingBluetooth.value = false;
	}
};
</script>

<template>
  <ToggleControl
    class="theme-transition"
    :name="bluetoothIcon"
    type="symbol"
    :label="t('components.BluetoothControl.toggle')"
    :pressed="isBluetoothOn"
    :is-active="isBluetoothOn"
    :is-loading="isTogglingBluetooth"
    :indicator="indicator"
    :badge="connectedDevicesCount"
    @click="toggleBT"
  />
</template>
