/**
 * La primera letra en mayúscula y el resto como viene.
 *
 * Para fechas formateadas por `Intl` («martes, 6 de octubre» → «Martes, 6 de
 * octubre»). No con el `capitalize` de CSS: ése pone mayúscula en **cada**
 * palabra, y en castellano «Martes, 6 De Octubre» está mal escrito. Con
 * `locale`, la mayúscula sigue las reglas de ese idioma (la «i» turca, por
 * ejemplo).
 */
export function capitalizeFirst(text: string, locale?: string): string {
	if (!text) return text;
	const [first = '', ...rest] = Array.from(text);
	return first.toLocaleUpperCase(locale) + rest.join('');
}
