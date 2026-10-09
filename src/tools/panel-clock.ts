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
	/** El día de la semana solo, para el renglón de arriba de la fecha: «domingo». */
	weekday: string;
	/** El número y el mes, para el renglón de abajo de la fecha: «22 de marzo». */
	dayMonth: string;
	/** La del globo y el nombre accesible: «domingo, 22 de marzo de 2026». */
	longDate: string;
}

const pad = (value: number) => value.toString().padStart(2, '0');

/**
 * Pone en mayúscula la primera letra del texto. En español `Intl` devuelve el
 * día y el mes en minúscula («domingo», «22 de marzo»); en la píldora del reloj
 * y en el globo se quieren empezados en mayúscula.
 *
 * Capitaliza sólo la primera letra real de la cadena, no cada palabra (eso haría
 * «22 De Marzo»): es lo contrario de lo que haría el `text-transform: capitalize`
 * del CSS. Respeta los alfabetos con mayúsculas y las cadenas vacías.
 */
export function capitalizeFirst(text: string): string {
	if (text.length === 0) return text;
	return text[0]?.toLocaleUpperCase() + text.slice(1);
}

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
		date: capitalizeFirst(
			formatter(locale, { weekday: 'long', day: 'numeric', month: 'long' }).format(date)
		),
		weekday: capitalizeFirst(formatter(locale, { weekday: 'long' }).format(date)),
		dayMonth: capitalizeFirst(formatter(locale, { day: 'numeric', month: 'long' }).format(date)),
		longDate: capitalizeFirst(
			formatter(locale, {
				weekday: 'long',
				day: 'numeric',
				month: 'long',
				year: 'numeric',
			}).format(date)
		),
	};
}
