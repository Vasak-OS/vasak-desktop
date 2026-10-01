import type { MenuDisposition, TrayMenu, TrayToggle } from '@/interfaces/tray';

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

/**
 * `parentEnabled` baja la habilitación de los ancestros: un submenú
 * deshabilitado deshabilita todo lo que tiene adentro, aunque cada hijo diga
 * `enabled: true` —el protocolo usa `enabled` como el control de activación—.
 */
export function trayMenuRows(
	items: readonly TrayMenu[] | null | undefined,
	depth = 0,
	parentEnabled = true
): TrayMenuRow[] {
	const rows: TrayMenuRow[] = [];
	for (const raw of items ?? []) {
		if (!raw.visible) continue;
		const item = parentEnabled ? raw : { ...raw, enabled: false };
		if (item.type === 'separator') {
			rows.push({ kind: 'separator', key: `sep-${depth}-${item.id}-${rows.length}` });
		} else if (item.type === 'submenu' || item.children?.length) {
			// Un submenú es un título aunque venga vacío: no es una acción. Mismo
			// criterio que `is_submenu` en `commands/tray.rs`.
			rows.push({ kind: 'caption', item, depth });
			rows.push(...trayMenuRows(item.children, depth + 1, item.enabled));
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

/** Cómo se llaman las teclas modificadoras de dbusmenu en una tecla. */
const MODIFIER_LABELS: Record<string, string> = {
	Control: 'Ctrl',
	Alt: 'Alt',
	Shift: 'Shift',
	Super: 'Super',
};

/**
 * El atajo de una entrada, como se escribe en un menú: `[["Control", "q"]]`
 * es «Ctrl+Q» y `[["Control", "Q"], ["Alt", "X"]]` es «Ctrl+Q, Alt+X». Sin
 * atajo, nada: no se reserva lugar.
 */
export function shortcutLabel(shortcut: readonly string[][] | undefined): string | undefined {
	const chords = (shortcut ?? [])
		.filter((chord) => chord.length > 0)
		.map((chord) =>
			chord
				.map((key, index) => {
					if (index < chord.length - 1) return MODIFIER_LABELS[key] ?? key;
					return key.length === 1 ? key.toUpperCase() : key;
				})
				.join('+')
		);
	return chords.length > 0 ? chords.join(', ') : undefined;
}

/**
 * El `role` y el `aria-checked` de una entrada. Una casilla indeterminada es
 * `mixed`; una opción de radio no admite `mixed` en ARIA, así que se anuncia
 * sin marcar y la diferencia la dice el dibujo.
 */
export function entryRole(item: TrayMenu): {
	role: 'menuitem' | 'menuitemcheckbox' | 'menuitemradio';
	checked?: 'true' | 'false' | 'mixed';
} {
	const toggle = item.toggle;
	if (!toggle) return { role: 'menuitem' };
	if (toggle.kind === 'radio') {
		return { role: 'menuitemradio', checked: toggle.state === 'on' ? 'true' : 'false' };
	}
	const checked = toggle.state === 'on' ? 'true' : toggle.state === 'off' ? 'false' : 'mixed';
	return { role: 'menuitemcheckbox', checked };
}

/** El indicador de la casilla o la opción de radio, del tema del sistema. */
export function toggleIconName(toggle: TrayToggle | undefined): string | undefined {
	if (!toggle) return undefined;
	const base = toggle.kind === 'radio' ? 'radio' : 'checkbox';
	switch (toggle.state) {
		case 'on':
			return `${base}-checked-symbolic`;
		case 'indeterminate':
			return `${base}-mixed-symbolic`;
		default:
			return `${base}-symbolic`;
	}
}

/** El tono y el icono de `disposition`. `normal` no llega. */
export const DISPOSITION_STYLE: Record<
	MenuDisposition,
	{ tone: 'info' | 'warning' | 'error'; icon: string }
> = {
	informative: { tone: 'info', icon: 'dialog-information-symbolic' },
	warning: { tone: 'warning', icon: 'dialog-warning-symbolic' },
	alert: { tone: 'error', icon: 'dialog-error-symbolic' },
};
