/**
 * Qué hace una tecla sobre los resultados de la búsqueda del menú.
 *
 * El menú escucha el teclado en todo el documento para que las flechas y Enter
 * funcionen con el foco en el campo de búsqueda. Pero los resultados son ítems
 * de menú de la librería (`DropdownMenuItem`), que atienden Enter ellos mismos
 * cuando tienen el foco —y le cancelan la acción por omisión—. Sin mirar eso, el
 * mismo Enter llegaba después acá y lanzaba **otra vez** una aplicación: la
 * marcada por las flechas, que no tiene por qué ser la enfocada.
 *
 * Lo que ya atendió otro (`defaultPrevented`) no se vuelve a atender. Va aparte
 * de la vista para poder probarlo sin montar nada.
 */

export type MenuKeyResult =
	| { kind: 'none' }
	| { kind: 'move'; index: number }
	| { kind: 'launch'; index: number };

const NEXT = new Set(['ArrowDown', 'ArrowRight']);
const PREVIOUS = new Set(['ArrowUp', 'ArrowLeft']);

export function resolveMenuKey(
	event: Pick<KeyboardEvent, 'key' | 'defaultPrevented'>,
	length: number,
	index: number
): MenuKeyResult {
	if (event.defaultPrevented || length === 0) return { kind: 'none' };

	if (NEXT.has(event.key)) return { kind: 'move', index: (index + 1) % length };
	if (PREVIOUS.has(event.key)) return { kind: 'move', index: (index - 1 + length) % length };
	if (event.key === 'Enter') return { kind: 'launch', index };

	return { kind: 'none' };
}
