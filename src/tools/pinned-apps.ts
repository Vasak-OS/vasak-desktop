/**
 * Las aplicaciones ancladas del panel (vasak-desktop#151, configurable desde
 * #251).
 *
 * La lista sale de la configuración —`panel.pinned`—, no está escrita acá. Lo
 * que vive en este archivo es el contrato de una entrada, la lista por defecto
 * (Configuración y Archivos, los dos accesos que el panel tuvo siempre) y las
 * dos funciones puras que resuelven «qué muestro» y «cómo se llama», para poder
 * probarlas sin montar Vue.
 *
 * **Semántica de `panel.pinned`:** ausente = los dos por defecto; presente
 * —aunque sea una lista vacía— = exactamente esa lista. Así apagar todos los
 * anclados es una decisión que se puede expresar (lista vacía) y distinta de
 * «nunca lo toqué» (ausente).
 */

export interface PinnedApp {
	/**
	 * La identidad del `.desktop`: su ruta de archivo. Es lo que consume
	 * `open_app` como `path` —abre ese archivo, lee su `Exec=` y lo lanza—, y es
	 * la misma forma con que todo el ecosistema identifica una aplicación
	 * (`menu.favorites`, `MenuApp.path`). Por eso anclar es guardar la ruta del
	 * `.desktop`, y lanzar es pasársela a `open_app`.
	 */
	id: string;
	/**
	 * El nombre visible. En una entrada de la configuración es el nombre de la
	 * aplicación tal como lo da el sistema; en las predeterminadas es una clave
	 * del catálogo que se traduce (ver `labelKey`).
	 */
	name: string;
	/** El nombre del icono en el tema. */
	icon: string;
	/**
	 * La línea `Exec` del `.desktop`. Es parte del contrato que comparte con la
	 * configuración, pero el escritorio no la usa para lanzar: `open_app` lee el
	 * `Exec=` del propio archivo. Queda como metadato (lo muestra la
	 * configuración al elegir, y es un dato más de la entrada).
	 */
	exec: string;
	/**
	 * Sólo en las predeterminadas: marca que `name` es una clave del catálogo y
	 * hay que traducirla. Una entrada de la configuración nunca lo trae —su
	 * `name` ya es el texto visible—, así que nunca se traduce un nombre de
	 * aplicación.
	 */
	labelKey?: boolean;
}

/**
 * El directorio estándar de los `.desktop` del sistema (XDG). Es donde el
 * paquete instala los de VasakOS y donde los busca el menú.
 */
const SYSTEM_APPLICATIONS = '/usr/share/applications';

/**
 * Los dos accesos de siempre, para cuando `panel.pinned` está ausente:
 * Configuración y Archivos. Su `id` es la ruta del `.desktop` del sistema, así
 * que se lanzan por el mismo camino que una entrada de la configuración, y su
 * `name` es una clave del catálogo para que queden traducidos.
 */
export const DEFAULT_PINNED: readonly PinnedApp[] = [
	{
		id: `${SYSTEM_APPLICATIONS}/vasak-settings.desktop`,
		name: 'views.panel.settingsAlt',
		icon: 'preferences-system',
		exec: 'vasak-settings',
		labelKey: true,
	},
	{
		id: `${SYSTEM_APPLICATIONS}/vasak-file-manager.desktop`,
		name: 'views.panel.filesAlt',
		icon: 'system-file-manager',
		exec: 'vasak-file-manager',
		labelKey: true,
	},
];

/**
 * La lista de anclados que corresponde a una sección `panel` de la
 * configuración.
 *
 * Ausente (o cualquier cosa que no sea un arreglo) = los dos por defecto;
 * un arreglo —aunque esté vacío— = exactamente ese arreglo. No se mezcla con los
 * por defecto: presente pisa, no suma.
 */
export function resolvePinnedApps(section: unknown): readonly PinnedApp[] {
	const pinned = (section as { pinned?: unknown } | null | undefined)?.pinned;
	return Array.isArray(pinned) ? (pinned as PinnedApp[]) : DEFAULT_PINNED;
}

/**
 * El texto visible de una entrada: traducido si es una predeterminada
 * (`labelKey`), literal si viene de la configuración.
 */
export function pinnedLabel(app: PinnedApp, t: (key: string) => string): string {
	return app.labelKey ? t(app.name) : app.name;
}
