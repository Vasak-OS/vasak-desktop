/**
 * Dónde vive el layout de widgets de cada pantalla.
 *
 * El escritorio se dibuja una vez por monitor (el backend abre una ventana por
 * salida y le pasa `?monitor=<salida>`; la principal es `desktop` y las demás
 * `desktop_1`, `desktop_2`…). Si todas leyeran y escribieran la misma lista, el
 * secundario mostraría los widgets del principal duplicados y, al mover uno en
 * una pantalla, pisaría el layout de la otra.
 *
 * Por eso el layout se guarda **por salida** dentro de la configuración central
 * (`vasak.conf`, vía el plugin config-manager, que desde 2.6.0 conserva las
 * claves que no conoce bajo `desktop`):
 *
 *  · la salida principal sigue en `desktop.widgets`, como siempre, para no
 *    tocar ni migrar lo que ya tiene configurado la gente;
 *  · cada salida secundaria tiene el suyo en
 *    `desktop.widgetsByMonitor[<salida>]`.
 */
import type { WidgetPlacement } from './catalog';

/** La salida principal: su layout vive en la clave heredada `desktop.widgets`. */
export const PRIMARY_MONITOR = 'desktop';

/**
 * Normaliza el parámetro `?monitor=` de la ventana a un nombre de salida.
 *
 * Sin el parámetro —o vacío— es la principal: así un escritorio abierto sin la
 * consulta no se queda sin layout.
 */
export function monitorLabel(raw: string | null | undefined): string {
	return raw && raw.length > 0 ? raw : PRIMARY_MONITOR;
}

/** Si una salida es la principal. */
export function isPrimaryMonitor(label: string): boolean {
	return label === PRIMARY_MONITOR;
}

/**
 * El layout guardado de una salida, o `undefined` si nunca se configuró.
 *
 * `undefined` no es lo mismo que una lista vacía: lo primero es «sin configurar»
 * (la principal arranca con la disposición de siempre; la secundaria, vacía) y
 * lo segundo es un escritorio que la persona dejó libre a propósito. La
 * distinción la resuelve `resolveLayout`.
 */
export function readMonitorWidgets(config: unknown, label: string): unknown {
	const desktop = (config as { desktop?: Record<string, unknown> } | null | undefined)?.desktop;
	if (isPrimaryMonitor(label)) return desktop?.widgets;
	const byMonitor = desktop?.widgetsByMonitor as Record<string, unknown> | undefined;
	return byMonitor?.[label];
}

/**
 * El config nuevo con el layout de esta salida, sin tocar el de las demás.
 *
 * Se devuelve el objeto entero porque así escribe el plugin: una salida sólo
 * cambia su propia clave y conserva `widgetsByMonitor` de las otras.
 */
export function withMonitorWidgets(
	config: unknown,
	label: string,
	placements: WidgetPlacement[]
): Record<string, unknown> {
	const current = (config as Record<string, unknown>) ?? {};
	const desktop = (current.desktop as Record<string, unknown>) ?? {};

	if (isPrimaryMonitor(label)) {
		return { ...current, desktop: { ...desktop, widgets: placements } };
	}

	const byMonitor = {
		...((desktop.widgetsByMonitor as Record<string, unknown>) ?? {}),
		[label]: placements,
	};
	return { ...current, desktop: { ...desktop, widgetsByMonitor: byMonitor } };
}
