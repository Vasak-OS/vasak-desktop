/**
 * Los dos estados del centro de control, sin Vue (vasak-desktop#175, ancla #174).
 *
 * - `notifications` (estado A): las notificaciones con todo el alto que sobra,
 *   la fila de interruptores redondos con «más», el brillo y el volumen.
 * - `settings` (estado B): las notificaciones resumidas en una línea que las
 *   vuelve a abrir, y los mosaicos de ajustes en su lugar.
 *
 * Lo que se decide acá es sólo cuál de los dos va; la conmutación es estado
 * local y CSS en la vista.
 */
export type ControlCenterMode = 'notifications' | 'settings';

/**
 * El estado con el que abre el centro.
 *
 * No se recuerda entre aperturas: decide si hay notificaciones. Sin ninguna no
 * hay nada que mostrar arriba, y abre en los ajustes.
 */
export function modeOnOpen(notificationCount: number): ControlCenterMode {
	return notificationCount > 0 ? 'notifications' : 'settings';
}

/**
 * El estado después de que cambia la cantidad de notificaciones con el centro
 * abierto.
 *
 * Sólo un caso mueve el estado: se fue la última (se limpiaron, o se borró la
 * única). Con la lista vacía el alto queda para los ajustes. Que llegue una
 * nueva estando en los ajustes **no** lo cambia: la línea resumen la cuenta, y
 * saltar solo le movería la pantalla a quien está tocando un mosaico.
 */
export function modeOnCountChange(
	previous: number,
	current: number,
	mode: ControlCenterMode
): ControlCenterMode {
	if (previous > 0 && current === 0) return 'settings';
	return mode;
}

/**
 * La clave del texto de la línea resumen, por cantidad de notificaciones y de
 * aplicaciones.
 *
 * El `t()` del escritorio no tiene plurales: van oraciones enteras por cada
 * combinación alcanzable (memoria `i18n-requirement`). Una notificación es
 * siempre de una aplicación, así que «1 de 2» no existe.
 */
export function summaryKey(notificationCount: number, appCount: number): string {
	const base = 'views.controlCenter.summary';
	if (notificationCount <= 0) return `${base}None`;
	if (notificationCount === 1) return `${base}OneOne`;
	return appCount === 1 ? `${base}OtherOne` : `${base}OtherOther`;
}

/** El texto de la línea resumen, con los números puestos. */
export function summaryText(
	t: (key: string) => string,
	notificationCount: number,
	appCount: number
): string {
	return t(summaryKey(notificationCount, appCount))
		.replace('{0}', String(notificationCount))
		.replace('{1}', String(appCount));
}
