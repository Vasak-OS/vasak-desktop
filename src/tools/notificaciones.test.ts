import { describe, expect, test } from 'bun:test';
import type { Notification } from '@/interfaces/notifications';
import { agruparNotificaciones, hayNotificacionesNuevas } from '@/tools/notificaciones';

function notificacion(parcial: Partial<Notification> & { id: number }): Notification {
	return {
		app_name: 'Telegram',
		app_icon: 'telegram',
		summary: 'Hola',
		body: '',
		timestamp: 100,
		seen: false,
		...parcial,
	};
}

describe('agruparNotificaciones', () => {
	test('junta por aplicación y cuenta', () => {
		const grupos = agruparNotificaciones([
			notificacion({ id: 1 }),
			notificacion({ id: 2, app_name: 'Correo', timestamp: 50 }),
			notificacion({ id: 3 }),
		]);

		expect(grupos.map((g) => [g.app_name, g.count])).toEqual([
			['Telegram', 2],
			['Correo', 1],
		]);
	});

	test('ordena por la notificación más reciente de cada grupo', () => {
		const grupos = agruparNotificaciones([
			notificacion({ id: 1, app_name: 'Vieja', timestamp: 10 }),
			notificacion({ id: 2, app_name: 'Nueva', timestamp: 900 }),
		]);

		expect(grupos.map((g) => g.app_name)).toEqual(['Nueva', 'Vieja']);
	});

	test('un grupo está sin leer si le queda alguna sin ver', () => {
		const [grupo] = agruparNotificaciones([
			notificacion({ id: 1, seen: true }),
			notificacion({ id: 2, seen: false }),
		]);

		expect(grupo.has_unread).toBe(true);
		expect(grupo.latest_timestamp).toBe(100);
	});

	/**
	 * Lo que hace que la lista no parpadee: borrar una notificación no puede
	 * cambiar la clave de los grupos que sobreviven, porque es lo que Vue mira
	 * para saber cuáles ya estaban dibujados.
	 */
	test('borrar una notificación no cambia la clave de los grupos que quedan', () => {
		const todas = [
			notificacion({ id: 1 }),
			notificacion({ id: 2 }),
			notificacion({ id: 3, app_name: 'Correo', timestamp: 50 }),
		];

		const antes = agruparNotificaciones(todas).map((g) => g.app_name);
		const despues = agruparNotificaciones(todas.filter((n) => n.id !== 2)).map((g) => g.app_name);

		expect(despues).toEqual(antes);
	});

	test('una aplicación sin notificaciones desaparece de la lista', () => {
		const todas = [notificacion({ id: 1 }), notificacion({ id: 2, app_name: 'Correo' })];
		const grupos = agruparNotificaciones(todas.filter((n) => n.app_name !== 'Correo'));

		expect(grupos.map((g) => g.app_name)).toEqual(['Telegram']);
	});

	test('sin notificaciones no hay grupos', () => {
		expect(agruparNotificaciones([])).toEqual([]);
	});
});

describe('hayNotificacionesNuevas', () => {
	test('una que no estaba antes cuenta como nueva', () => {
		const previas = [notificacion({ id: 1 })];
		expect(hayNotificacionesNuevas(previas, [notificacion({ id: 2 }), ...previas])).toBe(true);
	});

	/** El caso que hacía sonar la campanita al borrar: la foto trae lo que quedó. */
	test('borrar una no deja ninguna nueva', () => {
		const previas = [notificacion({ id: 1 }), notificacion({ id: 2 })];
		expect(hayNotificacionesNuevas(previas, [notificacion({ id: 1 })])).toBe(false);
	});

	test('la misma lista no trae nada nuevo', () => {
		const previas = [notificacion({ id: 1 }), notificacion({ id: 2 })];
		expect(hayNotificacionesNuevas(previas, [...previas])).toBe(false);
	});

	test('vaciarlas todas no trae nada nuevo', () => {
		expect(hayNotificacionesNuevas([notificacion({ id: 1 })], [])).toBe(false);
	});

	test('la primera notificación de la sesión es nueva', () => {
		expect(hayNotificacionesNuevas([], [notificacion({ id: 1 })])).toBe(true);
	});
});
