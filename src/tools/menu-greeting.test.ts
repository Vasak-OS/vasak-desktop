import { describe, expect, test } from 'bun:test';
import { greetingKey } from '@/tools/menu-greeting';

describe('el saludo por franja horaria', () => {
	test('la mañana va de 5 a 11', () => {
		expect(greetingKey(5)).toBe('views.menu.greeting.morning');
		expect(greetingKey(11)).toBe('views.menu.greeting.morning');
	});

	test('la tarde va de 12 a 18', () => {
		expect(greetingKey(12)).toBe('views.menu.greeting.afternoon');
		expect(greetingKey(18)).toBe('views.menu.greeting.afternoon');
	});

	test('la noche va de 19 a 4, cruzando la medianoche', () => {
		expect(greetingKey(19)).toBe('views.menu.greeting.evening');
		expect(greetingKey(23)).toBe('views.menu.greeting.evening');
		expect(greetingKey(0)).toBe('views.menu.greeting.evening');
		expect(greetingKey(4)).toBe('views.menu.greeting.evening');
	});
});
