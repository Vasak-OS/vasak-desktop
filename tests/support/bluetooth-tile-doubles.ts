/**
 * Los `@/…` de `BluetoothTile.vue` para montarlo en las pruebas: un estado de
 * Bluetooth fijo (prendido, sin dispositivos) y un `logError` que anota.
 */
import { ref } from 'vue';

export const logged: unknown[][] = [];

export function useBluetoothState() {
	return { isBluetoothOn: ref(true), connectedDevicesCount: ref(0) };
}

export function logError(...args: unknown[]): void {
	logged.push(args);
}

/** Sin modo avión ni bloqueo de rfkill (vasak-desktop#180). */
export function useAirplaneMode() {
	return { bluetoothBlocked: ref(false), unblockRadios: async () => {} };
}
