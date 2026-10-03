<script setup lang="ts">
/**
 * La vista radial del Bluetooth y de la red (vasak-desktop#132), y la lista.
 *
 * Abre en la **órbita**: lo conectado al centro y sus datos alrededor
 * (`DeviceOrbit` de vue-libvasak; qué satélites y en qué orden lo arma
 * `connection-orbit.ts`). La lista no se va: la vista radial **se suma**, no la
 * reemplaza —decisión del 02/10/2026—. Muestra lo conectado, y el satélite de
 * arriba («Buscar dispositivos», «Ver redes») pasa a la lista para elegir otro;
 * desde la lista, «Volver» trae la órbita.
 *
 * Abajo, como en la referencia, el selector **Wi-Fi | Bluetooth** y el
 * encendido de la radio de la pestaña. El encendido es el `SwitchToggle` que ya
 * tenían los dos applets para eso mismo, y no un botón redondo nuevo:
 * interruptor donde hoy hay interruptor.
 *
 * # De dónde salen los datos
 *
 * - **Bluetooth:** `tauri-plugin-bluetooth-manager`, el primer dispositivo
 *   conectado del adaptador por omisión. La batería es `DeviceInfo.battery`, que
 *   va ausente cuando el dispositivo no la publica: entonces el satélite no se
 *   dibuja. El perfil de audio lo da el escritorio, que lo lee de PipeWire
 *   (`get_bluetooth_audio_profile` y `bluetooth-audio-profile-changed`).
 * - **Red:** `tauri-plugin-network-manager` por `network.service.ts`.
 *
 * Esconder el applet no destruye el webview: al volver a mostrarse
 * (`applet-shown`) se vuelve a la órbita y se pide todo de nuevo.
 *
 * # Angosto
 *
 * `DeviceOrbit` apila sola cuando la órbita no entra, y el pie pasa a dos
 * renglones por consulta de contenedor: una columna por vez.
 */
import { invoke } from '@tauri-apps/api/core';
import {
	type AdapterInfo,
	type DeviceInfo,
	getConnectedDevices,
	getDefaultAdapter,
	getDeviceInfo,
	toggleBluetooth,
} from '@vasakgroup/plugin-bluetooth-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	DeviceOrbit,
	SegmentedControl,
	SwitchToggle,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import BluetoothControlArea from '@/components/areas/bluetooth/BluetoothControlArea.vue';
import NetworkControlArea from '@/components/areas/network/NetworkControlArea.vue';
import {
	getCurrentNetworkState,
	getWirelessEnabled,
	isWirelessAvailable,
	type NetworkInfo,
	setWirelessEnabled,
} from '@/services/network.service';
import {
	type BluetoothAudioProfile,
	bluetoothCenter,
	bluetoothSatellites,
	networkCenter,
	networkSatellites,
} from '@/tools/connection-orbit';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

export type ConnectionTab = 'wifi' | 'bluetooth';

const props = defineProps<{
	/** La pestaña con la que abre: la del botón del panel que lo abrió. */
	initialTab: ConnectionTab;
	/** El applet que la contiene, para saber cuándo vuelve a mostrarse. */
	applet: string;
}>();

const { t } = useI18n();

const tab = ref<ConnectionTab>(props.initialTab);
const view = ref<'radial' | 'list'>('radial');
/** La lista de Bluetooth abre buscando cuando se llegó desde «Buscar». */
const scanOnOpen = ref(false);

// ── Bluetooth ──────────────────────────────────────────────────────────────
const adapter = ref<AdapterInfo | null>(null);
const device = ref<DeviceInfo | null>(null);
const profile = ref<BluetoothAudioProfile | null>(null);
const togglingBluetooth = ref(false);
const bluetoothOn = computed(() => Boolean(adapter.value?.powered));

async function refreshProfile() {
	const address = device.value?.address;
	if (!address) {
		profile.value = null;
		return;
	}
	try {
		profile.value = await invoke<BluetoothAudioProfile | null>('get_bluetooth_audio_profile', {
			address,
		});
	} catch (error) {
		profile.value = null;
		logError('[ConnectionsArea] no se pudo leer el perfil de audio:', error);
	}
}

async function refreshBluetooth() {
	try {
		adapter.value = await getDefaultAdapter();
		const connected = adapter.value?.powered ? await getConnectedDevices(adapter.value.path) : [];
		const first = connected[0] ?? null;
		// `getDeviceInfo` trae la batería al día; la lista de conectados puede
		// venir de antes.
		device.value = first ? await getDeviceInfo(first.path).catch(() => first) : null;
	} catch (error) {
		device.value = null;
		logError('[ConnectionsArea] no se pudo leer el Bluetooth:', error);
	}
	await refreshProfile();
}

async function toggleBluetoothRadio() {
	togglingBluetooth.value = true;
	try {
		await toggleBluetooth();
		await refreshBluetooth();
	} catch (error) {
		logError('[ConnectionsArea] no se pudo alternar el Bluetooth:', error);
	} finally {
		togglingBluetooth.value = false;
	}
}

useSharedEvent('bluetooth-change', () => {
	void refreshBluetooth();
});

useSharedEvent<BluetoothAudioProfile>('bluetooth-audio-profile-changed', (payload) => {
	if (!payload || !device.value) return;
	if (payload.address.toUpperCase() !== device.value.address.toUpperCase()) return;
	profile.value = payload;
});

// ── Red ────────────────────────────────────────────────────────────────────
const network = ref<NetworkInfo | null>(null);
const wifiAvailable = ref(true);
const wifiEnabled = ref(false);
const togglingWifi = ref(false);

async function refreshNetwork() {
	try {
		wifiAvailable.value = await isWirelessAvailable();
		wifiEnabled.value = wifiAvailable.value ? await getWirelessEnabled() : false;
		network.value = await getCurrentNetworkState();
	} catch (error) {
		network.value = null;
		logError('[ConnectionsArea] no se pudo leer la red:', error);
	}
}

async function toggleWifiRadio() {
	if (!wifiAvailable.value) return;
	togglingWifi.value = true;
	try {
		await setWirelessEnabled(!wifiEnabled.value);
		await refreshNetwork();
	} catch (error) {
		logError('[ConnectionsArea] no se pudo alternar el Wi-Fi:', error);
	} finally {
		togglingWifi.value = false;
	}
}

useSharedEvent<NetworkInfo>('network-changed', (payload) => {
	if (payload) network.value = payload;
	void refreshNetwork();
});

// ── La vista ───────────────────────────────────────────────────────────────
const tabs = computed(() => [
	{ value: 'wifi' as const, label: t('components.ConnectionsArea.wifi'), icon: 'network-wireless' },
	{
		value: 'bluetooth' as const,
		label: t('components.ConnectionsArea.bluetooth'),
		icon: 'bluetooth',
	},
]);

const center = computed(() =>
	tab.value === 'bluetooth' ? bluetoothCenter(device.value, t) : networkCenter(network.value, t)
);

const satellites = computed(() =>
	tab.value === 'bluetooth'
		? bluetoothSatellites(device.value, profile.value, t, { disabled: !bluetoothOn.value })
		: networkSatellites(network.value, t, { disabled: !wifiEnabled.value })
);

const emptyLabel = computed(() => {
	if (tab.value === 'bluetooth') {
		return bluetoothOn.value
			? t('components.ConnectionsArea.noDevice')
			: t('components.ConnectionsArea.bluetoothOff');
	}
	if (!wifiAvailable.value) return t('components.ConnectionsArea.wifiUnavailable');
	return wifiEnabled.value
		? t('components.ConnectionsArea.noNetwork')
		: t('components.ConnectionsArea.wifiOff');
});

const emptyIcon = computed(() =>
	tab.value === 'bluetooth' ? 'bluetooth-disabled' : 'network-wireless-offline'
);

const radioOn = computed(() => (tab.value === 'bluetooth' ? bluetoothOn.value : wifiEnabled.value));
const radioBusy = computed(() =>
	tab.value === 'bluetooth' ? togglingBluetooth.value : togglingWifi.value
);
const radioLabel = computed(() =>
	tab.value === 'bluetooth'
		? t('components.ConnectionsArea.bluetoothRadio')
		: t('components.ConnectionsArea.wifiRadio')
);
const radioDisabled = computed(
	() => radioBusy.value || (tab.value === 'wifi' && !wifiAvailable.value)
);

function chooseTab(next: ConnectionTab | null) {
	if (next) tab.value = next;
}

function toggleRadio() {
	if (tab.value === 'bluetooth') void toggleBluetoothRadio();
	else void toggleWifiRadio();
}

function onSelect(id: string) {
	if (id === 'scan') {
		scanOnOpen.value = true;
		view.value = 'list';
	} else if (id === 'networks') {
		scanOnOpen.value = false;
		view.value = 'list';
	}
}

function backToOrbit() {
	scanOnOpen.value = false;
	view.value = 'radial';
	void refreshAll();
}

async function refreshAll() {
	await Promise.all([refreshBluetooth(), refreshNetwork()]);
}

useSharedEvent<{ applet?: string }>('applet-shown', (payload) => {
	if (payload?.applet !== props.applet) return;
	tab.value = props.initialTab;
	view.value = 'radial';
	scanOnOpen.value = false;
	void refreshAll();
});

onMounted(() => {
	void refreshAll();
});
</script>

<template>
  <div class="@container flex h-full min-h-0 min-w-0 flex-col gap-3" data-connections-view :data-view="view">
    <template v-if="view === 'radial'">
      <div class="min-h-0 flex-1">
        <DeviceOrbit
          :label="tab === 'bluetooth' ? t('components.ConnectionsArea.bluetooth') : t('components.ConnectionsArea.wifi')"
          :center="center"
          :satellites="satellites"
          :empty-label="emptyLabel"
          :empty-icon="emptyIcon"
          :orbit-key="tab"
          @select="onSelect"
        />
      </div>

      <!-- El pie: el selector centrado y el encendido a la derecha; angosto,
           en dos renglones. -->
      <div class="grid grid-cols-1 items-center justify-items-center gap-2 @md:grid-cols-[1fr_auto_1fr]">
        <span class="hidden @md:block" aria-hidden="true" />
        <SegmentedControl
          :model-value="tab"
          :options="tabs"
          :label="t('components.ConnectionsArea.tabs')"
          @update:model-value="chooseTab"
        />
        <SwitchToggle
          class="@md:justify-self-end"
          :label="radioLabel"
          :model-value="radioOn"
          :disabled="radioDisabled"
          @update:model-value="toggleRadio"
        />
      </div>
    </template>

    <template v-else>
      <div class="flex min-w-0 items-center">
        <ActionButton
          :label="t('components.ConnectionsArea.back')"
          icon="go-previous"
          icon-type="symbol"
          variant="ghost"
          @click="backToOrbit"
        />
      </div>
      <div class="min-h-0 flex-1 overflow-hidden">
        <BluetoothControlArea v-if="tab === 'bluetooth'" :scan-on-mount="scanOnOpen" />
        <NetworkControlArea v-else hide-x />
      </div>
    </template>
  </div>
</template>
