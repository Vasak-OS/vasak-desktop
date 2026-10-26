/** El icono del botón de menú del panel cuando `panel.menuIcon` no dice otro. */
export const DEFAULT_MENU_ICON = 'start-here';

/**
 * El icono del botón de menú según la sección `panel`: el nombre de `menuIcon`
 * si es un texto no vacío, y el de siempre en cualquier otro caso.
 */
export function resolveMenuIcon(section: unknown): string {
	const icon = (section as { menuIcon?: unknown } | null | undefined)?.menuIcon;
	return typeof icon === 'string' && icon.trim() !== '' ? icon.trim() : DEFAULT_MENU_ICON;
}
