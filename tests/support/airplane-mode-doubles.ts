/**
 * Los `@/…` de los mosaicos de Modo avión, Wi-Fi y Bluetooth para montarlos en
 * las pruebas (vasak-desktop#180). El componible del modo avión, el del Wi-Fi y
 * el bus de eventos son los de verdad: lo que se dobla es el `invoke` de Tauri
 * y los eventos, desde la prueba. El estado de la red y el de Bluetooth son
 * fijos: conectado y prendido, para ver que el bloqueo de rfkill gana.
 */
import { ref } from 'vue';

export { useAirplaneMode } from '../../src/tools/composables/useAirplaneMode';
export { useWifiToggle } from '../../src/tools/composables/useWifiToggle';
export { useSharedEvent } from '../../src/tools/event.bus';

export function useNetworkState() {
	return {
		networkState: ref({ is_connected: true, ssid: 'casa', name: 'casa' }),
		networkIconName: ref('network-wireless-signal-excellent-symbolic'),
		getCurrentNetwork: async () => {},
	};
}

export function useBluetoothState() {
	return { isBluetoothOn: ref(true), connectedDevicesCount: ref(0) };
}

export function logError(): void {}
