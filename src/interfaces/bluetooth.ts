import type { AdapterInfo } from '@vasakgroup/plugin-bluetooth-manager';

export interface BluetoothState {
	connectedDevices: any[];
	availableDevices: any[];
	defaultAdapter: AdapterInfo | null;
	connectedDevicesCount: number;
	bluetoothIcon: string;
}
