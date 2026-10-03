/**
 * Lo que dice la píldora del reloj (vasak-desktop#151).
 *
 * Aparte del componente para poder probarlo sin montar nada: la hora con dos
 * dígitos, la fecha corta de abajo y la larga del globo, en el idioma de la
 * sesión.
 */

export interface ClockParts {
	/** «09:05». */
	time: string;
	hour: string;
	minute: string;
	/** La de abajo de la hora: «domingo, 22 de marzo». */
	date: string;
	/** La del globo y el nombre accesible: «domingo, 22 de marzo de 2026». */
	longDate: string;
}

const pad = (value: number) => value.toString().padStart(2, '0');

/**
 * Un idioma que `Intl` acepta: el catálogo dice «es» o «en», pero si viniera
 * algo raro no puede tirar abajo el reloj.
 */
function formatter(
	locale: string | undefined,
	options: Intl.DateTimeFormatOptions
): Intl.DateTimeFormat {
	try {
		return new Intl.DateTimeFormat(locale || undefined, options);
	} catch {
		return new Intl.DateTimeFormat(undefined, options);
	}
}

export function clockParts(date: Date, locale?: string): ClockParts {
	const hour = pad(date.getHours());
	const minute = pad(date.getMinutes());
	return {
		time: `${hour}:${minute}`,
		hour,
		minute,
		date: formatter(locale, { weekday: 'long', day: 'numeric', month: 'long' }).format(date),
		longDate: formatter(locale, {
			weekday: 'long',
			day: 'numeric',
			month: 'long',
			year: 'numeric',
		}).format(date),
	};
}
