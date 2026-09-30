import type { AdapterInfo, BluetoothChange } from '@vasakgroup/plugin-bluetooth-manager';
import type { Ref } from 'vue';

export interface BluetoothState {
	connectedDevices: any[];
	availableDevices: any[];
	defaultAdapter: AdapterInfo | null;
	connectedDevicesCount: number;
	bluetoothIcon: string;
}

export interface BluetoothStateRefs {
	availableDevices: Ref<any[]>;
	connectedDevices: Ref<any[]>;
	defaultAdapter: Ref<AdapterInfo | null>;
}

/**
 * El payload del evento `bluetooth-change`, con el tipo del complemento.
 *
 * Deja afuera `change_type`, el nombre de hasta la 2.1 que el complemento
 * todavía manda pero se va en la próxima mayor: así nada lo puede volver a
 * leer sin que el typecheck lo marque.
 */
export type BluetoothChangePayload = Pick<BluetoothChange, 'changeType' | 'data'>;

export interface BluetoothComposableOptions {
	getIcon: (iconName: string) => Promise<string>;
}
