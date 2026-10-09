import { describe, expect, test } from 'bun:test';
import { COMPACT_FROM, FULL_FROM, panelDensity, showsNames, showsNumbers } from './panel-density';

describe('cuánto texto entra en el panel', () => {
	test('en una pantalla ancha, todo', () => {
		expect(panelDensity(1920)).toBe('full');
		expect(panelDensity(FULL_FROM)).toBe('full');
	});

	test('más angosta, los nombres largos se pliegan al icono', () => {
		expect(panelDensity(1280)).toBe('compact');
		expect(showsNames('compact')).toBe(false);
		expect(showsNumbers('compact')).toBe(true);
	});

	test('más angosta todavía, también los números', () => {
		expect(panelDensity(COMPACT_FROM - 1)).toBe('tight');
		expect(panelDensity(600)).toBe('tight');
		expect(showsNumbers('tight')).toBe(false);
		expect(showsNames('tight')).toBe(false);
	});

	test('el ancho completo muestra nombres y números', () => {
		expect(showsNames('full')).toBe(true);
		expect(showsNumbers('full')).toBe(true);
	});
});
