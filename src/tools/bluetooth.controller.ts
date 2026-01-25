import { ref, computed, onMounted, onUnmounted, Ref } from 'vue';
import { getDefaultAdapter, getConnectedDevicesCount } from '@vasakgroup/plugin-bluetooth-manager';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@/interfaces/event';
import type { BluetoothStateRefs, BluetoothChangePayload, BluetoothComposableOptions } from '@/interfaces/bluetooth';

const findDeviceIndex = (devices: any[], path: string) => {
	return devices.findIndex((d) => d.path === path);
};

const deviceExists = (devices: any[], path: string) => {
	return devices.some((d) => d.path === path);
};

const addDeviceIfNotExists = (devices: Ref<any[]>, device: any) => {
	if (!deviceExists(devices.value, device.path)) {
		devices.value.push(device);
	}
};

const moveDevice = (from: Ref<any[]>, to: Ref<any[]>, device: any) => {
	const index = findDeviceIndex(from.value, device.path);
	if (index !== -1) {
		from.value.splice(index, 1);
		addDeviceIfNotExists(to, device);
	}
};

const handleAdapterPropertyChanged = (data: any, state: BluetoothStateRefs) => {
	if (
		state.defaultAdapter.value &&
    data.path === state.defaultAdapter.value.path
	) {
		state.defaultAdapter.value = data;
	}
};

const handleDeviceAdded = (data: any, state: BluetoothStateRefs) => {
	addDeviceIfNotExists(state.availableDevices, data);
};

const handleDeviceRemoved = (data: any, state: BluetoothStateRefs) => {
	state.availableDevices.value = state.availableDevices.value.filter(
		(d) => d.path !== data.path
	);
	state.connectedDevices.value = state.connectedDevices.value.filter(
		(d) => d.path !== data.path
	);
};

const updateDeviceInAvailable = (
	deviceIndex: number,
	data: any,
	state: BluetoothStateRefs
) => {
	if (data.connected) {
		moveDevice(state.availableDevices, state.connectedDevices, data);
	} else {
		state.availableDevices.value[deviceIndex] = data;
	}
};

const updateDeviceInConnected = (
	connectedIndex: number,
	data: any,
	state: BluetoothStateRefs
) => {
	if (data.connected) {
		state.connectedDevices.value[connectedIndex] = data;
	} else {
		moveDevice(state.connectedDevices, state.availableDevices, data);
	}
};

const handleDeviceUpdate = (data: any, state: BluetoothStateRefs) => {
	const deviceIndex = findDeviceIndex(state.availableDevices.value, data.path);

	if (deviceIndex !== -1) {
		updateDeviceInAvailable(deviceIndex, data, state);
		return;
	}

	const connectedIndex = findDeviceIndex(
		state.connectedDevices.value,
		data.path
	);
	if (connectedIndex !== -1) {
		updateDeviceInConnected(connectedIndex, data, state);
	}
};

const handleDeviceDisconnected = (data: any, state: BluetoothStateRefs) => {
	moveDevice(state.connectedDevices, state.availableDevices, data);
};

export const applyBluetoothChange = (
	payload: BluetoothChangePayload,
	state: BluetoothStateRefs
) => {
	const { change_type, data } = payload;

	const handlers: Record<
    string,
    (data: any, state: BluetoothStateRefs) => void
  > = {
  	'adapter-property-changed': handleAdapterPropertyChanged,
  	'device-added': handleDeviceAdded,
  	'device-removed': handleDeviceRemoved,
  	'device-connected': handleDeviceUpdate,
  	'device-property-changed': handleDeviceUpdate,
  	'device-disconnected': handleDeviceDisconnected,
  };

	const handler = handlers[change_type];
	if (handler) {
		handler(data, state);
	}
};

export const resolveBluetoothIconName = (
	powered: boolean | undefined | null,
	connectedCount: number
) => {
	if (!powered) return 'bluetooth-disabled-symbolic';
	return connectedCount > 0
		? 'bluetooth-active-symbolic'
		: 'bluetooth-symbolic';
};

/**
 * Composable shared logic for Bluetooth components
 */
export function useBluetoothState(options: BluetoothComposableOptions) {
	const connectedDevices: Ref<any[]> = ref([]);
	const availableDevices: Ref<any[]> = ref([]);
	const bluetoothIcon: Ref<string> = ref('');
	const defaultAdapter = ref<any>(null);
	const connectedDevicesCount: Ref<number> = ref(0);
	const unlistenBluetooth: Ref<UnlistenFn | null> = ref(null);

	const isBluetoothOn = computed(() => defaultAdapter.value?.powered);

	const handleBluetoothChange = async (event: any): Promise<void> => {
		applyBluetoothChange(event.payload, {
			availableDevices,
			connectedDevices,
			defaultAdapter,
		});
		await getBluetoothIcon();
	};

	const getBluetoothIcon = async (): Promise<void> => {
		try {
			connectedDevicesCount.value = await getConnectedDevicesCount(
				defaultAdapter.value?.path as string
			);
			const iconName = resolveBluetoothIconName(
				isBluetoothOn.value,
				connectedDevicesCount.value
			);
			bluetoothIcon.value = await options.getIcon(iconName);
		} catch (error) {
			console.error('Error loading bluetooth icon:', error);
		}
	};

	const initializeBluetoothState = async (): Promise<void> => {
		defaultAdapter.value = await getDefaultAdapter();
		await getBluetoothIcon();
		connectedDevicesCount.value = await getConnectedDevicesCount(
			defaultAdapter.value?.path as string
		);
		unlistenBluetooth.value = await listen(
			'bluetooth-change',
			handleBluetoothChange
		);
	};

	onMounted(initializeBluetoothState);

	onUnmounted(() => {
		if (unlistenBluetooth.value) {
			unlistenBluetooth.value();
		}
	});

	return {
		connectedDevices,
		availableDevices,
		bluetoothIcon,
		defaultAdapter,
		connectedDevicesCount,
		isBluetoothOn,
		getBluetoothIcon,
	};
}
