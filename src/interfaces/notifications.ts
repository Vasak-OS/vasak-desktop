export interface Notification {
	id: number;
	app_name: string;
	app_icon: string;
	summary: string;
	body: string;
	timestamp: number;
	seen: boolean;
	urgency?: string;
	actions?: string[];
	hints?: { [key: string]: string };
}

export interface NotificationGroupData {
	app_name: string;
	app_icon: string;
	notifications: Notification[];
	count: number;
	latest_timestamp: number;
	has_unread: boolean;
}

/**
 * Lo que manda Rust cuando el demonio avisa un cambio: la lista entera, en un
 * solo evento.
 *
 * Eran cuatro formas —`added`, `removed`, `batch_update`, `cleared`—, pero la
 * única que se emitía era `cleared` seguida de un `batch_update` con todo de
 * nuevo. Como son dos eventos, la lista quedaba vacía entre uno y otro y
 * borrar una notificación hacía que las demás repitieran la animación de
 * entrada. Con una sola foto se reemplaza de una sola vez.
 */
export type NotificationDelta = { action: 'snapshot'; items: Notification[] };
