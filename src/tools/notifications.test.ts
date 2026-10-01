import { describe, expect, test } from 'bun:test';
import type { Notification } from '@/interfaces/notifications';
import { containsNewNotifications, groupNotifications } from '@/tools/notifications';

function notification(partial: Partial<Notification> & { id: number }): Notification {
	return {
		app_name: 'Telegram',
		app_icon: 'telegram',
		summary: 'Hola',
		body: '',
		timestamp: 100,
		seen: false,
		...partial,
	};
}

describe('groupNotifications', () => {
	test('junta por aplicación y cuenta', () => {
		const groups = groupNotifications([
			notification({ id: 1 }),
			notification({ id: 2, app_name: 'Correo', timestamp: 50 }),
			notification({ id: 3 }),
		]);

		expect(groups.map((g) => [g.app_name, g.count])).toEqual([
			['Telegram', 2],
			['Correo', 1],
		]);
	});

	test('ordena por la notificación más reciente de cada grupo', () => {
		const groups = groupNotifications([
			notification({ id: 1, app_name: 'Vieja', timestamp: 10 }),
			notification({ id: 2, app_name: 'Nueva', timestamp: 900 }),
		]);

		expect(groups.map((g) => g.app_name)).toEqual(['Nueva', 'Vieja']);
	});

	test('un grupo está sin leer si le queda alguna sin ver', () => {
		const [group] = groupNotifications([
			notification({ id: 1, seen: true }),
			notification({ id: 2, seen: false }),
		]);

		expect(group.has_unread).toBe(true);
		expect(group.latest_timestamp).toBe(100);
	});

	/**
	 * Lo que hace que la lista no parpadee: borrar una notificación no puede
	 * cambiar la clave de los grupos que sobreviven, porque es lo que Vue mira
	 * para saber cuáles ya estaban dibujados.
	 */
	test('borrar una notificación no cambia la clave de los grupos que quedan', () => {
		const all = [
			notification({ id: 1 }),
			notification({ id: 2 }),
			notification({ id: 3, app_name: 'Correo', timestamp: 50 }),
		];

		const before = groupNotifications(all).map((g) => g.app_name);
		const after = groupNotifications(all.filter((n) => n.id !== 2)).map((g) => g.app_name);

		expect(after).toEqual(before);
	});

	test('una aplicación sin notificaciones desaparece de la lista', () => {
		const all = [notification({ id: 1 }), notification({ id: 2, app_name: 'Correo' })];
		const groups = groupNotifications(all.filter((n) => n.app_name !== 'Correo'));

		expect(groups.map((g) => g.app_name)).toEqual(['Telegram']);
	});

	test('sin notificaciones no hay grupos', () => {
		expect(groupNotifications([])).toEqual([]);
	});
});

describe('containsNewNotifications', () => {
	test('una que no estaba antes cuenta como nueva', () => {
		const previous = [notification({ id: 1 })];
		expect(containsNewNotifications(previous, [notification({ id: 2 }), ...previous])).toBe(true);
	});

	/** El caso que hacía sonar la campanita al borrar: la foto trae lo que quedó. */
	test('borrar una no deja ninguna nueva', () => {
		const previous = [notification({ id: 1 }), notification({ id: 2 })];
		expect(containsNewNotifications(previous, [notification({ id: 1 })])).toBe(false);
	});

	test('la misma lista no trae nada nuevo', () => {
		const previous = [notification({ id: 1 }), notification({ id: 2 })];
		expect(containsNewNotifications(previous, [...previous])).toBe(false);
	});

	test('vaciarlas todas no trae nada nuevo', () => {
		expect(containsNewNotifications([notification({ id: 1 })], [])).toBe(false);
	});

	test('la primera notificación de la sesión es nueva', () => {
		expect(containsNewNotifications([], [notification({ id: 1 })])).toBe(true);
	});
});
