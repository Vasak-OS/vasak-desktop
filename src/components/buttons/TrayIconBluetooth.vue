
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { TrayIconButton } from '@vasakgroup/vue-libvasak';
import { computed, ref } from 'vue';
import { toggleApplet } from '@/services/window.service';
import { useBluetoothState } from '@/tools/bluetooth.controller';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const { isBluetoothOn, connectedDevicesCount } = useBluetoothState({
	getIcon: async () => '',
});

const bluetoothIcon = computed(() => {
	if (!isBluetoothOn.value) return 'bluetooth-disabled-symbolic';
	return connectedDevicesCount.value > 0 ? 'bluetooth-active-symbolic' : 'bluetooth-symbolic';
});

const button = ref<unknown>(null);
const { openClasses } = useOpenApplet('bluetooth');

const toggleBluetooth = async (): Promise<void> => {
	try {
		await toggleApplet('bluetooth', button.value);
	} catch (error) {
		logError('Error toggling bluetooth applet:', error);
	}
};
</script>

<template>
  <TrayIconButton
    ref="button"
    :name="bluetoothIcon"
    :tooltip="isBluetoothOn
      ? t('components.TrayIconBluetooth.statusOn')
      : t('components.TrayIconBluetooth.statusOff')"
    :alt="t('components.TrayIconBluetooth.iconAlt')"
    :badge="isBluetoothOn && connectedDevicesCount > 0 ? connectedDevicesCount : null"
    :icon-class="{ 'filter brightness-75': !isBluetoothOn }"
    :custom-class="openClasses"
    @click="toggleBluetooth"
  />
</template>
