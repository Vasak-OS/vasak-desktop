<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { toggleBluetooth } from '@vasakgroup/plugin-bluetooth-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, type Ref, ref } from 'vue';
import { useBluetoothState } from '@/tools/bluetooth.controller';
import { useIcon } from '@/tools/composables/useReactiveIcon';
import { logError } from '@/utils/logger';
import ToggleControl from '../forms/ToggleControl.vue';

const { t } = useI18n();

const isTogglingBluetooth: Ref<boolean> = ref(false);

const { isBluetoothOn, connectedDevicesCount } = useBluetoothState({
	getIcon: async () => '',
});

const bluetoothIcon = useIcon(
	computed(() => {
		if (!isBluetoothOn.value) return 'bluetooth-disabled-symbolic';
		return connectedDevicesCount.value > 0 ? 'bluetooth-active-symbolic' : 'bluetooth-symbolic';
	})
);

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
  <div class="theme-transition relative inline-block">
    <!-- Indicador de estado -->
    <div
      class="absolute top-1 right-1 w-3 h-3 rounded-full transition-all duration-300"
      :class="{
        'bg-blue-400 animate-pulse': isBluetoothOn && connectedDevicesCount > 0,
        'bg-blue-400': isBluetoothOn && connectedDevicesCount === 0,
        'bg-gray-400': !isBluetoothOn,
      }"
    ></div>

    <!-- Badge de dispositivos conectados -->
    <div
      v-if="connectedDevicesCount > 0"
      class="absolute bottom-1 right-1 bg-primary text-tx-main text-xs rounded-full w-4 h-4 flex items-center justify-center font-bold"
    >
      {{ connectedDevicesCount }}
    </div>

    <ToggleControl
      :icon="bluetoothIcon"
      :alt="t('components.BluetoothControl.iconAlt')"
      :tooltip="t('components.BluetoothControl.toggle')"
      :is-active="isBluetoothOn"
      :is-loading="isTogglingBluetooth"
      :custom-class="{
        'ring-2 ring-blue-400/50': isBluetoothOn,
      }"
      @click="toggleBT"
    />
  </div>
</template>
