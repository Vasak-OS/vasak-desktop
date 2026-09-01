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
export function agruparNotificaciones(
	notificaciones: readonly Notification[]
): NotificationGroupData[] {
	const grupos = new Map<string, NotificationGroupData>();

	for (const notificacion of notificaciones) {
		const aplicacion = notificacion.app_name;
		let grupo = grupos.get(aplicacion);

		if (!grupo) {
			grupo = {
				app_name: aplicacion,
				app_icon: notificacion.app_icon,
				notifications: [],
				count: 0,
				latest_timestamp: 0,
				has_unread: false,
			};
			grupos.set(aplicacion, grupo);
		}

		grupo.notifications.push(notificacion);
		grupo.count = grupo.notifications.length;
		grupo.latest_timestamp = Math.max(grupo.latest_timestamp, notificacion.timestamp);
		grupo.has_unread = grupo.has_unread || !notificacion.seen;
	}

	return [...grupos.values()].sort((a, b) => b.latest_timestamp - a.latest_timestamp);
}

/**
 * Si la foto que llegó trae alguna notificación que antes no estaba.
 *
 * El panel sacude la campanita cuando llega algo, y con esto sabe cuándo. Antes
 * se guiaba por «vino una lista con cosas adentro», que es verdad en toda foto:
 * la campanita se sacudía también al **borrar** una notificación, porque las
 * que quedaban llegaban igual en el mismo evento.
 */
export function hayNotificacionesNuevas(
	previas: readonly Notification[],
	siguientes: readonly Notification[]
): boolean {
	const conocidas = new Set(previas.map((n) => n.id));
	return siguientes.some((n) => !conocidas.has(n.id));
}
