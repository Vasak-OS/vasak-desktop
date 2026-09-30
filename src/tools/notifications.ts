import type { Notification, NotificationGroupData } from '@/interfaces/notifications';

/**
 * Las notificaciones agrupadas por aplicación, de la más reciente a la más
 * vieja.
 *
 * La clave del grupo es el nombre de la aplicación, y de eso depende que la
 * lista no parpadee: es lo que Vue compara para saber qué grupo ya estaba
 * dibujado. Mientras la aplicación siga teniendo notificaciones, su tarjeta es
 * la misma aunque el objeto que la describe sea nuevo.
 */
export function groupNotifications(
	notifications: readonly Notification[]
): NotificationGroupData[] {
	const groups = new Map<string, NotificationGroupData>();

	for (const notification of notifications) {
		const app = notification.app_name;
		let group = groups.get(app);

		if (!group) {
			group = {
				app_name: app,
				app_icon: notification.app_icon,
				notifications: [],
				count: 0,
				latest_timestamp: 0,
				has_unread: false,
			};
			groups.set(app, group);
		}

		group.notifications.push(notification);
		group.count = group.notifications.length;
		group.latest_timestamp = Math.max(group.latest_timestamp, notification.timestamp);
		group.has_unread = group.has_unread || !notification.seen;
	}

	return [...groups.values()].sort((a, b) => b.latest_timestamp - a.latest_timestamp);
}

/**
 * Si la foto que llegó trae alguna notificación que antes no estaba.
 *
 * El panel sacude la campanita cuando llega algo, y con esto sabe cuándo. Antes
 * se guiaba por «vino una lista con cosas adentro», que es verdad en toda foto:
 * la campanita se sacudía también al **borrar** una notificación, porque las
 * que quedaban llegaban igual en el mismo evento.
 */
export function containsNewNotifications(
	previous: readonly Notification[],
	next: readonly Notification[]
): boolean {
	const known = new Set(previous.map((n) => n.id));
	return next.some((n) => !known.has(n.id));
}
