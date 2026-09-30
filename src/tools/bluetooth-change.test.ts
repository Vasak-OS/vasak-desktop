import { describe, expect, test } from 'bun:test';
import { ref } from 'vue';
import type { BluetoothChangePayload, BluetoothStateRefs } from '@/interfaces/bluetooth';
import { applyBluetoothChange } from './bluetooth.controller';

const adapterPath = '/org/bluez/hci0';

const device = (path: string, connected: boolean) => ({
	path,
	address: '00:11:22:33:44:55',
	connected,
	adapter: adapterPath,
});

const emptyState = (): BluetoothStateRefs => ({
	availableDevices: ref<any[]>([]),
	connectedDevices: ref<any[]>([]),
	defaultAdapter: ref<any>({ path: adapterPath, powered: false }),
});

/**
 * El payload lleva sólo `changeType`, a propósito: `change_type` es el nombre
 * de hasta la 2.1 del complemento y se va en la próxima mayor. Si el applet lo
 * siguiera leyendo, estas pruebas no encontrarían manejador y fallarían.
 */
const change = (changeType: string, data: any): BluetoothChangePayload => ({ changeType, data });

describe('applyBluetoothChange lee `changeType`', () => {
	test('un dispositivo nuevo sin conectar va a los disponibles', () => {
		const state = emptyState();
		applyBluetoothChange(change('device-added', device('/dev_a', false)), state);

		expect(state.availableDevices.value.map((d) => d.path)).toEqual(['/dev_a']);
		expect(state.connectedDevices.value).toEqual([]);
	});

	test('un dispositivo nuevo ya conectado va a los conectados', () => {
		const state = emptyState();
		applyBluetoothChange(change('device-added', device('/dev_a', true)), state);

		expect(state.connectedDevices.value.map((d) => d.path)).toEqual(['/dev_a']);
		expect(state.availableDevices.value).toEqual([]);
	});

	test('al conectarse pasa de disponibles a conectados', () => {
		const state = emptyState();
		state.availableDevices.value = [device('/dev_a', false)];
		applyBluetoothChange(change('device-connected', device('/dev_a', true)), state);

		expect(state.availableDevices.value).toEqual([]);
		expect(state.connectedDevices.value.map((d) => d.path)).toEqual(['/dev_a']);
	});

	test('al desconectarse vuelve a los disponibles', () => {
		const state = emptyState();
		state.connectedDevices.value = [device('/dev_a', true)];
		applyBluetoothChange(change('device-disconnected', device('/dev_a', false)), state);

		expect(state.connectedDevices.value).toEqual([]);
		expect(state.availableDevices.value.map((d) => d.path)).toEqual(['/dev_a']);
	});

	test('un dispositivo quitado sale de las dos listas', () => {
		const state = emptyState();
		state.availableDevices.value = [device('/dev_a', false)];
		state.connectedDevices.value = [device('/dev_b', true)];
		applyBluetoothChange(change('device-removed', { path: '/dev_a' }), state);
		applyBluetoothChange(change('device-removed', { path: '/dev_b' }), state);

		expect(state.availableDevices.value).toEqual([]);
		expect(state.connectedDevices.value).toEqual([]);
	});

	test('un cambio del adaptador por defecto reemplaza su estado', () => {
		const state = emptyState();
		applyBluetoothChange(
			change('adapter-property-changed', { path: adapterPath, powered: true }),
			state
		);

		expect(state.defaultAdapter.value?.powered).toBe(true);
	});

	test('un cambio de otro adaptador no toca el por defecto', () => {
		const state = emptyState();
		applyBluetoothChange(
			change('adapter-property-changed', { path: '/org/bluez/hci1', powered: true }),
			state
		);

		expect(state.defaultAdapter.value?.powered).toBe(false);
	});

	test('una clase de cambio sin manejador no toca nada', () => {
		const state = emptyState();
		applyBluetoothChange(change('dbus-error', device('/dev_a', false)), state);

		expect(state.availableDevices.value).toEqual([]);
		expect(state.connectedDevices.value).toEqual([]);
	});
});
