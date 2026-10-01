import { describe, expect, test } from 'bun:test';
import {
	calculateVolumePercentage,
	getVolumeIconName,
	volumePercentageClass,
} from '@/utils/volume';

describe('el color del porcentaje del volumen', () => {
	test('silenciado, el color del error del esquema', () => {
		expect(volumePercentageClass(true, 50)).toBe('text-status-error');
		// Aunque esté arriba: silenciado manda.
		expect(volumePercentageClass(true, 95)).toBe('text-status-error');
	});

	test('por encima del 80 %, el del éxito', () => {
		expect(volumePercentageClass(false, 81)).toBe('text-status-success');
		expect(volumePercentageClass(false, 80)).toBe('');
	});

	test('nunca un color de la paleta de Tailwind', () => {
		for (const [muted, percentage] of [
			[true, 0],
			[false, 10],
			[false, 100],
		] as const) {
			expect(volumePercentageClass(muted, percentage)).not.toMatch(/-(?:red|green)-\d{3}/);
		}
	});
});

describe('el icono y el porcentaje', () => {
	test('el icono sigue al nivel', () => {
		expect(getVolumeIconName(true, 70)).toBe('audio-volume-muted-symbolic');
		expect(getVolumeIconName(false, 0)).toBe('audio-volume-muted-symbolic');
		expect(getVolumeIconName(false, 20)).toBe('audio-volume-low-symbolic');
		expect(getVolumeIconName(false, 50)).toBe('audio-volume-medium-symbolic');
		expect(getVolumeIconName(false, 90)).toBe('audio-volume-high-symbolic');
	});

	test('el porcentaje se mide contra el rango', () => {
		expect(calculateVolumePercentage({ current: 0, min: 0, max: 200, is_muted: false }, 100)).toBe(
			50
		);
	});
});
