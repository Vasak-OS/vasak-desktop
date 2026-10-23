/**
 * Lo que importa `ClipboardView.vue` desde `@/…` (vasak-desktop#217).
 *
 * Los servicios son **los de verdad**: llaman `invoke`, y el `invoke` de mentira
 * de `mount-sfc.ts` anota qué comando llega al backend. Así la prueba ve el
 * nombre del comando, no una función doblada que podría no llamar a nada. El
 * filtro es puro, también el de verdad. Sólo se doblan el bus de eventos —para
 * poder disparar `window-shown` a mano— y el logger.
 */
export { clipboardClear, clipboardHistory, clipboardPick, hideClipboard } from '../../src/services/clipboard.service';
export { type ClipboardEntry, filterEntries } from '../../src/tools/clipboard-filter';

/** Los manejadores que registró la vista; la prueba los dispara. */
export const shownHandlers: Array<() => void> = [];
export function useSharedEvent(_event: string, handler: () => void): void {
	shownHandlers.push(handler);
}

export const logged: unknown[][] = [];
export function logError(...args: unknown[]): void {
	logged.push(args);
}
