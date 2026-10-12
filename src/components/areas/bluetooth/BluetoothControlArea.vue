<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import {
	type AdapterInfo,
	connectDevice,
	disconnectDevice,
	getAvailableDevices,
	getConnectedDevices,
	getDefaultAdapter,
	getDeviceInfo,
	startScan,
	stopScan,
	toggleBluetooth,
} from '@vasakgroup/plugin-bluetooth-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton, LoadingState, SwitchToggle, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import BluetoothDeviceCard from '@/components/cards/BluetoothDeviceCard.vue';
import { applyBluetoothChange, resolveBluetoothIconName } from '@/tools/bluetooth.controller';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const props = defineProps<{
	/**
	 * Buscar apenas se abre: la vista radial pasa a la lista con «Buscar
	 * dispositivos» (`ConnectionsArea`).
	 */
	scanOnMount?: boolean;
}>();

const connectedDevices = ref<any[]>([]);
const availableDevices = ref<any[]>([]);
const isTogglingBluetooth = ref(false);
const defaultAdapter = ref<AdapterInfo | null>(null);
const connectedDevicesCount = ref(0);
const loading = ref(true);
/**
 * Hay un escaneo ABIERTO. No es el cronómetro de antes: se enciende al empezar
 * a buscar y sólo se apaga al cerrar la vista (`onUnmounted`) o al apagar el
 * Bluetooth. Mientras tanto los `device-added` del streaming suman en vivo.
 */
const scanning = ref(false);
const connectingPath = ref<string | null>(null);

const toggleBT = async () => {
	isTogglingBluetooth.value = true;
	try {
		await toggleBluetooth();
		await refreshDevices();
	} catch (error) {
		logError('Error alternando Bluetooth:', error);
	} finally {
		isTogglingBluetooth.value = false;
	}
};

const isBluetoothOn = computed(() => defaultAdapter.value?.powered);

const handleBluetoothChange = async (payload: any) => {
	applyBluetoothChange(payload, {
		availableDevices,
		connectedDevices,
		defaultAdapter,
	});
};

const refreshDevices = async () => {
	if (!defaultAdapter.value) return;
	loading.value = true;
	try {
		connectedDevices.value = await getConnectedDevices(defaultAdapter.value.path);
		availableDevices.value = await getAvailableDevices(defaultAdapter.value.path);
		connectedDevicesCount.value = connectedDevices.value.length;
	} catch (e) {
		console.error('Error refreshing devices:', e);
		connectedDevices.value = [];
		availableDevices.value = [];
	}
	loading.value = false;
};

const scanDevices = async () => {
	if (!defaultAdapter.value) return;
	// Si ya hay un escaneo abierto, no se vuelve a arrancar: un segundo
	// `startScan` sobre el mismo adaptador no suma nada.
	if (scanning.value) return;
	scanning.value = true;
	try {
		// Primero se siembra lo conocido/emparejado una sola vez; después se
		// deja el escaneo ABIERTO y `handleBluetoothChange` va sumando los
		// `device-added` a medida que BlueZ los anuncia —incluidos los BLE que
		// anuncian lento, que la foto con cronómetro se perdía—.
		await refreshDevices();
		await startScan(defaultAdapter.value.path);
	} catch (e) {
		logError('Error iniciando el escaneo de dispositivos:', e);
		scanning.value = false;
	}
};

const stopScanning = async () => {
	if (!defaultAdapter.value || !scanning.value) return;
	scanning.value = false;
	try {
		await stopScan(defaultAdapter.value.path);
	} catch (e) {
		logError('Error deteniendo el escaneo de dispositivos:', e);
	}
};

const bluetoothIcon = computed(() => {
	connectedDevicesCount.value = connectedDevices.value.length;
	return resolveBluetoothIconName(isBluetoothOn.value, connectedDevicesCount.value);
});

onMounted(async () => {
	defaultAdapter.value = await getDefaultAdapter();
	// Cuando se abre buscando, el escaneo siembra él mismo la lista; si no, se
	// siembra sin abrir escaneo.
	if (props.scanOnMount && defaultAdapter.value?.powered) {
		await scanDevices();
	} else {
		await refreshDevices();
	}
});

// Nunca se corta por tiempo: el escaneo abierto se cierra al desmontar la
// vista. Sin esto, BlueZ seguiría descubriendo con la vista ya cerrada.
onUnmounted(() => {
	void stopScanning();
});

// Si el Bluetooth se apaga, el adaptador deja de descubrir: se apaga también el
// indicador para que la rueda no quede girando sin que se busque nada.
watch(isBluetoothOn, (on) => {
	if (!on) scanning.value = false;
});

useSharedEvent('bluetooth-change', handleBluetoothChange);

const connect = async (device: any) => {
	connectingPath.value = device.path;
	try {
		await connectDevice(device.path);
		for (let i = 0; i < 30; i++) {
			await new Promise((r) => setTimeout(r, 500));
			try {
				const info = await getDeviceInfo(device.path);
				if (info?.connected) break;
			} catch {
				// keep polling
			}
		}
		await refreshDevices();
	} catch (e) {
		logError('Error connecting to device:', e);
	}
	connectingPath.value = null;
};
const disconnect = async (device: any) => {
	try {
		await disconnectDevice(device.path);
		await refreshDevices();
	} catch (e) {
		logError('Error disconnecting device:', e);
	}
};
</script>

<template>
  <div class="flex flex-col h-full">
    <div class="flex flex-wrap items-center gap-y-2 mb-4">
      <SwitchToggle :label="t('components.BluetoothControl.toggle')"
        :model-value="isBluetoothOn || false"
        :disabled="isTogglingBluetooth"
        size="medium"
        class="mr-2"
        @update:model-value="toggleBT"
      />
      <ThemeIcon :name="bluetoothIcon" :size="32" alt="Bluetooth" class="mr-3" />
      <span class="min-w-0 flex-1 break-words font-bold text-2xl">Bluetooth</span>
      <!-- El botón de la librería: mientras busca, la rueda ocupa el lugar
           del icono y el ancho no cambia. `view-refresh` y no
           `refreshstructure`, que no es un nombre del estándar de freedesktop
           y no lo trae cualquier tema. -->
      <ActionButton
        label=""
        icon="view-refresh"
        :icon-alt="t('components.BluetoothControlArea.scanAlt')"
        :title="t('components.BluetoothControlArea.scanAlt')"
        variant="secondary"
        :loading="scanning"
        :disabled="!isBluetoothOn"
        @click="scanDevices"
      />
    </div>
    <div v-if="loading" class="text-center px-6 flex-1">{{ t('common.loading') }}</div>
    <div v-else class="flex-1 flex gap-4 flex-col">
      <div class="flex-1 flex flex-col overflow-hidden">
        <div class="mb-4 font-semibold text-xl">{{ t('components.BluetoothControlArea.available') }}</div>
        <div class="flex-1 overflow-y-auto">
          <!-- El escaneo queda abierto: mientras busca, la espera sin medida de
               la librería explica por qué la lista todavía puede crecer. -->
          <LoadingState
            v-if="scanning"
            :label="t('components.BluetoothControlArea.scanning')"
            size="sm"
            class="mb-2 px-1.5"
          />
          <div
            v-if="availableDevices.length === 0 && !scanning"
            class="text-tx-muted text-label-m px-1.5 text-center"
          >
            {{ t('components.BluetoothControlArea.noneAvailable') }}
          </div>
          <ul v-else class="list-none p-0 m-0">
            <li v-for="dev in availableDevices" :key="dev.path">
              <BluetoothDeviceCard
                :device="dev"
                :action-label="t('components.BluetoothControlArea.connect')"
                :is-connecting="connectingPath === dev.path"
                @action="connect(dev)"
              />
            </li>
          </ul>
        </div>
      </div>
      <div class="flex-1 flex flex-col overflow-hidden">
        <div class="mb-4 font-semibold text-xl">{{ t('components.BluetoothControlArea.connectedDevices') }}</div>
        <div class="flex-1 overflow-y-auto">
          <div
            v-if="connectedDevices.length === 0"
            class="text-tx-muted text-label-m px-1.5 text-center"
          >
            {{ t('components.BluetoothControlArea.noneConnected') }}
          </div>
          <ul v-else class="list-none p-0 m-0">
            <li v-for="dev in connectedDevices" :key="dev.path">
              <BluetoothDeviceCard
                :device="dev"
                :action-label="t('components.BluetoothControlArea.disconnect')"
                connected
                @action="disconnect(dev)"
              />
            </li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</template>
