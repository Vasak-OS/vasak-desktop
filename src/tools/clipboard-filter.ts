/** Una entrada del historial del portapapeles, como la da el backend. */
export interface ClipboardEntry {
	id: number;
	text: string;
}

/**
 * Las entradas cuyo texto contiene `query`, sin distinguir mayúsculas. Con la
 * búsqueda vacía, todas. El orden se conserva (el backend ya las da de la más
 * reciente a la más vieja).
 */
export function filterEntries(entries: ClipboardEntry[], query: string): ClipboardEntry[] {
	const needle = query.trim().toLowerCase();
	if (needle === '') return entries;
	return entries.filter((entry) => entry.text.toLowerCase().includes(needle));
}
