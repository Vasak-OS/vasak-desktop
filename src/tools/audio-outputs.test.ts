import { describe, expect, test } from 'bun:test';
import type { AudioDevice } from '@/interfaces/audio-device';
import { currentOutput, outputIcon, outputLabel } from './audio-outputs';

const device = (over: Partial<AudioDevice>): AudioDevice => ({
	id: '1',
	name: 'Audio interno',
	description: 'alsa_output.pci-0000_00_1f.3.analog-stereo',
	is_default: false,
	volume: 0.5,
	...over,
});

describe('la salida que suena', () => {
	test('es la que PipeWire tiene por omisión', () => {
		const devices = [device({ id: 'a' }), device({ id: 'b', is_default: true })];
		expect(currentOutput(devices)?.id).toBe('b');
	});

	test('sin ninguna por omisión, la primera: ahí va a caer el audio', () => {
		expect(currentOutput([device({ id: 'a' }), device({ id: 'b' })])?.id).toBe('a');
	});

	test('sin salidas, ninguna', () => {
		expect(currentOutput([])).toBeNull();
	});
});

describe('cómo se muestra', () => {
	test('sin la capa de audio que la publica', () => {
		expect(outputLabel(device({ name: 'Audio interno Estéreo analógico PipeWire' }))).toBe(
			'Audio interno Estéreo analógico'
		);
		expect(outputLabel(device({ name: 'ALSA  HDMI  PulseAudio' }))).toBe('HDMI');
	});

	test('si el nombre queda vacío, el del sumidero', () => {
		expect(outputLabel(device({ name: 'PipeWire', description: 'hdmi-stereo' }))).toBe(
			'hdmi-stereo'
		);
	});

	test('auriculares por Bluetooth o por nombre, parlante lo demás', () => {
		expect(outputIcon(device({ description: 'bluez_output.00_1B_66.1' }))).toBe('audio-headphones');
		expect(outputIcon(device({ name: 'USB Headset' }))).toBe('audio-headphones');
		expect(outputIcon(device({ name: 'Auriculares' }))).toBe('audio-headphones');
		expect(outputIcon(device({}))).toBe('audio-speakers');
		expect(outputIcon(null)).toBe('audio-speakers');
	});
});
