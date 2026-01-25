import type { AdapterInfo } from '@vasakgroup/plugin-bluetooth-manager';
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

export interface BluetoothChangePayload {
	change_type: string;
	data: any;
}

export interface BluetoothComposableOptions {
	getIcon: (iconName: string) => Promise<string>;
}
