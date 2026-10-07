<script setup lang="ts">
/**
 * El mosaico de Bluetooth (vasak-desktop#175): el cuerpo lo prende y lo apaga,
 * la línea de estado dice cuántos dispositivos hay conectados y la flecha abre
 * el detalle dentro del bloque.
 *
 * Con el Bluetooth bloqueado por rfkill —el modo avión, la tecla del teclado—
 * se ve apagado en el acto, sin esperar a BlueZ (vasak-desktop#180).
 */
import { toggleBluetooth } from '@vasakgroup/plugin-bluetooth-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { QuickSettingsTile } from '@vasakgroup/vue-libvasak';
import { computed, ref } from 'vue';
import { useAirplaneMode } from '@/tools/composables/useAirplaneMode';
import { useBluetoothState } from '@/tools/bluetooth.controller';
import { logError } from '@/utils/logger';

const emit = defineEmits<{ open: [] }>();
const { t } = useI18n();
const { isBluetoothOn, connectedDevicesCount } = useBluetoothState({ getIcon: async () => '' });
const { bluetoothBlocked, unblockRadios } = useAirplaneMode();
const toggling = ref(false);
const isOn = computed(() => Boolean(isBluetoothOn.value) && !bluetoothBlocked.value);

const icon = computed(() => {
	if (!isOn.value) return 'bluetooth-disabled';
	return connectedDevicesCount.value > 0 ? 'bluetooth-active' : 'bluetooth';
});

const status = computed(() => {
	if (!isOn.value) return t('components.ControlCenterTiles.disabled');
	const count = connectedDevicesCount.value;
	if (count === 0) return t('components.ControlCenterTiles.enabled');
	return t(
		count === 1
			? 'components.ControlCenterTiles.devicesOne'
			: 'components.ControlCenterTiles.devicesOther'
	).replace('{0}', String(count));
});

async function onActivate(): Promise<void> {
	// Un doble clic mientras el primero sigue en curso no manda otro pedido.
	if (toggling.value) return;
	toggling.value = true;
	try {
		// Bloqueado por rfkill, tocarlo lo desbloquea; BlueZ prende el
		// adaptador solo si estaba prendido antes del bloqueo.
		if (bluetoothBlocked.value) await unblockRadios('bluetooth');
		else await toggleBluetooth();
	} catch (error) {
		logError('[BluetoothTile] no se pudo alternar el Bluetooth:', error);
	} finally {
		toggling.value = false;
	}
}
</script>

<template>
  <QuickSettingsTile
    :icon="icon"
    :title="t('components.ControlCenterTiles.bluetooth')"
    :status="status"
    :active="isOn"
    :loading="toggling"
    detail
    @activate="onActivate"
    @detail="emit('open')"
  />
</template>
