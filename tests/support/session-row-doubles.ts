/**
 * Lo que importa `SessionActionsRow.vue` desde `@/…` (vasak-desktop#190).
 *
 * Los servicios son **los de verdad**: llaman `invoke`, y el `invoke` de
 * mentira de `mount-sfc.ts` anota qué comando se pidió. Así la prueba ve el
 * nombre del comando que llega al backend, no una función doblada que podría
 * no llamar a nada. Sólo el logger es doble: el de verdad reemplaza
 * `console.error` al construirse.
 */
export { isLockScreenAvailable, lockScreen } from '../../src/services/session-lock.service';
export { hideControlCenter, toggleSessionPopup } from '../../src/services/window.service';
export { CONFIRMED_ACTIONS, dialogAction } from '../../src/tools/session-row';

export const logged: unknown[][] = [];
export function logError(...args: unknown[]): void {
	logged.push(args);
}
