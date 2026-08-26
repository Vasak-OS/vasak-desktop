
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed } from 'vue';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import { toggleBluetoothApplet } from '@/services/window.service';
import { useBluetoothState } from '@/tools/bluetooth.controller';
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const { isBluetoothOn, connectedDevicesCount } = useBluetoothState({
	getIcon: async () => '',
});

const bluetoothIcon = useSymbol(
	computed(() => {
		if (!isBluetoothOn.value) return 'bluetooth-disabled-symbolic';
		return connectedDevicesCount.value > 0 ? 'bluetooth-active-symbolic' : 'bluetooth-symbolic';
	})
);

const toggleBluetooth = async (): Promise<void> => {
	try {
		await toggleBluetoothApplet();
	} catch (error) {
		logError('Error toggling bluetooth applet:', error);
	}
};
</script>

<template>
  <TrayIconButton
    :icon="bluetoothIcon"
    :tooltip="isBluetoothOn
      ? t('components.TrayIconBluetooth.statusOn')
      : t('components.TrayIconBluetooth.statusOff')"
    :alt="t('components.TrayIconBluetooth.iconAlt')"
    :badge="isBluetoothOn && connectedDevicesCount > 0 ? connectedDevicesCount : null"
    :icon-class="{ 'filter brightness-75': !isBluetoothOn }"
    @click="toggleBluetooth"
  />
</template>
