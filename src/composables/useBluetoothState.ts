import { ref, computed, onMounted, onUnmounted, Ref } from 'vue';
import { getDefaultAdapter, getConnectedDevicesCount } from '@vasakgroup/plugin-bluetooth-manager';
import { listen } from '@tauri-apps/api/event';
import { applyBluetoothChange, resolveBluetoothIconName } from '@/tools/bluetooth.controller';
import type { UnlistenFn } from '@/interfaces/event';

interface BluetoothComposableOptions {
	getIcon: (iconName: string) => Promise<string>;
}

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
