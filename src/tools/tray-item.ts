import type { LauncherBadge, LauncherEntryView, TrayIcon, TrayItem } from '@/interfaces/tray';

/**
 * Qué se dibuja de un elemento de la bandeja, decidido sin DOM para poder
 * probarlo. La regla de todo el archivo: **lo que no viene no se dibuja**. Un
 * elemento sin insignia no tiene un hueco donde iría la insignia, uno sin
 * progreso no tiene una barra vacía, uno sin globo no tiene un `title` vacío.
 */

/** De dónde sale un icono: el mapa de bits de la aplicación o el tema. */
export type IconSource = { kind: 'pixmap'; data: string } | { kind: 'theme'; name: string };

/**
 * El mapa de bits gana: es lo que dibujó la aplicación, no algo que se le
 * parece. El nombre queda para cuando no mandó mapa de bits.
 */
export function iconSource(icon: TrayIcon | undefined): IconSource | null {
	if (icon?.data) return { kind: 'pixmap', data: icon.data };
	if (icon?.name) return { kind: 'theme', name: icon.name };
	return null;
}

/** Si el elemento pide atención: por su estado o por `urgent` del lanzador. */
export function needsAttention(item: TrayItem): boolean {
	return item.status === 'NeedsAttention' || item.launcher?.urgent === true;
}

/**
 * El icono principal.
 *
 * Con `NeedsAttention`, el de atención si lo mandó; si no, `AttentionMovieName`
 * cuando es un nombre del tema (una ruta absoluta es una animación que no se
 * reproduce acá). Fuera de ese estado, el de siempre. `urgent` del lanzador no
 * cambia el icono: no dice cuál poner, sólo que hay que mirar.
 */
export function mainIcon(item: TrayItem): IconSource | null {
	if (item.status === 'NeedsAttention') {
		const attention = iconSource(item.attention_icon);
		if (attention) return attention;
		const movie = item.attention_movie_name;
		if (movie && !movie.startsWith('/')) return { kind: 'theme', name: movie };
	}
	return iconSource({ name: item.icon_name, data: item.icon_data });
}

/** La insignia superpuesta, si viene. */
export function overlayIcon(item: TrayItem): IconSource | null {
	return iconSource(item.overlay_icon);
}

/** El nombre del elemento para quien no ve el icono. */
export function itemName(item: TrayItem): string {
	return item.tooltip?.title || item.title || item.id;
}

/**
 * El texto del globo: título y descripción en dos líneas.
 *
 * Va como globo nativo (`title`), y no con el `Tooltip` de la librería: el
 * panel es una superficie de capa del alto de la barra y un globo dibujado
 * adentro quedaría recortado. El nativo lo abre GTK como ventana emergente
 * propia. Por eso el icono del globo no se dibuja: un globo nativo es sólo
 * texto.
 */
export function tooltipText(item: TrayItem): string | undefined {
	const title = item.tooltip?.title || item.title;
	const description = item.tooltip?.description;
	const text = [title, description].filter(Boolean).join('\n');
	return text || undefined;
}

/** El contador para dibujar, o nada. Más de 99 es «99+»: no entra otra cosa. */
export function countLabel(count: number | undefined): string | undefined {
	if (count === undefined || !Number.isFinite(count) || count <= 0) return undefined;
	return count > 99 ? '99+' : String(Math.trunc(count));
}

/**
 * El progreso en porcentaje para `ProgressBar`, o nada. El backend ya lo
 * recorta a [0, 1]; acá se vuelve a recortar porque es barato y la vista no
 * tiene por qué creerle a nadie.
 */
export function progressPercent(progress: number | undefined): number | undefined {
	if (progress === undefined || !Number.isFinite(progress)) return undefined;
	return Math.round(Math.min(1, Math.max(0, progress)) * 100);
}

/**
 * Normaliza un id de `.desktop` o un `app-id` de ventana para compararlos:
 * sin el dominio invertido, sin signos y en minúsculas. Mismo criterio que
 * `desktop_names` en `tray/launcher_entry.rs`. Ninguno de los dos trae la
 * extensión (el backend la saca del `app_uri`), y no se le saca un
 * `.desktop` final: `org.telegram.desktop` es el id entero de Telegram.
 */
export function normaliseAppId(id: string): string {
	const DOMAIN = new Set(['org', 'com', 'io', 'net', 'dev', 'app', 'me', 'de', 'fr', 'es', 'ar']);
	const segments = id.toLowerCase().split('.').filter(Boolean);
	if (segments.length > 1 && DOMAIN.has(segments[0])) segments.shift();
	return segments.join('').replace(/[^\p{L}\p{N}]/gu, '');
}

/**
 * Lo que publicó por `LauncherEntry` la aplicación de una ventana, buscado por
 * su `app-id` (que en Wayland es el id de su `.desktop`).
 */
export function launcherForApp(
	entries: readonly LauncherEntryView[],
	appId: string | undefined
): LauncherBadge | undefined {
	if (!appId) return undefined;
	const wanted = normaliseAppId(appId);
	if (!wanted) return undefined;
	const entry = entries.find((e) => normaliseAppId(e.desktop_id) === wanted);
	if (!entry) return undefined;
	const { desktop_id: _, ...badge } = entry;
	return badge;
}
