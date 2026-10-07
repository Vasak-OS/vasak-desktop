/**
 * La fila de sesión del centro de control (vasak-desktop#190, ancla #174):
 * qué acciones van, en qué orden, y cuáles preguntan antes.
 *
 * Bloquear va primero y no pregunta: no se pierde nada, y se vuelve con la
 * contraseña. Las otras tres abren el diálogo de sesión, que es su
 * confirmación. Suspender no está en la fila: sigue en el diálogo de sesión.
 */
import type { PowerAction } from '@vasakgroup/vue-libvasak';

export const SESSION_ROW_ACTIONS = [
	'lock',
	'logout',
	'reboot',
	'poweroff',
] as const satisfies readonly PowerAction[];

export type SessionRowAction = (typeof SESSION_ROW_ACTIONS)[number];

/** Las que pasan por el diálogo de sesión antes de hacerse. */
export const CONFIRMED_ACTIONS: readonly PowerAction[] = SESSION_ROW_ACTIONS.filter(
	(action) => action !== 'lock'
);

/**
 * El nombre que entiende el diálogo de sesión (`SessionPopupView`), o `null`
 * si la acción no pasa por él. El diálogo dice `shutdown` donde la librería
 * dice `poweroff`.
 */
export function dialogAction(action: PowerAction): string | null {
	if (action === 'lock') return null;
	return action === 'poweroff' ? 'shutdown' : action;
}
