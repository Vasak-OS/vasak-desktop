import type { TrayMenu } from '@/interfaces/tray';

/**
 * El menú de un icono de la bandeja, listo para dibujar como menú contextual.
 *
 * El programa lo manda como árbol por `com.canonical.dbusmenu`. Acá se aplana en
 * filas de tres clases: una entrada que se puede tocar, un separador, y el
 * título de un submenú con sus entradas debajo, sangradas. Un submenú no se
 * despliega aparte: en un menú de bandeja son cortos, y abrir una segunda
 * superficie para dos opciones es más lento que leerlas.
 *
 * Lo que el programa marca `visible: false` no se dibuja. El backend mide el
 * alto con el mismo criterio (`tray_menu_size` en `commands/tray.rs`): si una
 * de las dos cambia, la otra también, o el menú queda cortado o con aire.
 */
export type TrayMenuRow =
	| { kind: 'item'; item: TrayMenu; depth: number }
	| { kind: 'separator'; key: string }
	| { kind: 'caption'; item: TrayMenu; depth: number };

export function trayMenuRows(items: readonly TrayMenu[] | undefined, depth = 0): TrayMenuRow[] {
	const rows: TrayMenuRow[] = [];
	for (const item of items ?? []) {
		if (!item.visible) continue;
		if (item.type === 'separator') {
			rows.push({ kind: 'separator', key: `sep-${depth}-${item.id}-${rows.length}` });
		} else if (item.children?.length) {
			rows.push({ kind: 'caption', item, depth });
			rows.push(...trayMenuRows(item.children, depth + 1));
		} else {
			rows.push({ kind: 'item', item, depth });
		}
	}
	return rows;
}

/** Si el menú tiene algo que tocar. */
export function hasActions(rows: readonly TrayMenuRow[]): boolean {
	return rows.some((row) => row.kind === 'item');
}

/**
 * A qué entrada lleva una tecla, como en cualquier menú: flechas arriba y
 * abajo con vuelta en los extremos, Inicio y Fin. Las deshabilitadas se saltan.
 *
 * `enabled` es una por entrada tocable, en orden; `from` la que tiene el foco,
 * o -1 si ninguna. Devuelve `null` si la tecla no mueve o no hay adónde.
 */
export function menuFocusTarget(
	enabled: readonly boolean[],
	from: number,
	key: string
): number | null {
	const candidates = enabled.flatMap((on, index) => (on ? [index] : []));
	if (candidates.length === 0) return null;

	switch (key) {
		case 'Home':
			return candidates[0];
		case 'End':
			return candidates[candidates.length - 1];
		case 'ArrowDown':
			return candidates.find((index) => index > from) ?? candidates[0];
		case 'ArrowUp':
			return (
				[...candidates].reverse().find((index) => index < from) ?? candidates[candidates.length - 1]
			);
		default:
			return null;
	}
}
