/**
 * El saludo por franja horaria del encabezado hero (vasak-desktop#203).
 *
 * Devuelve la **clave** de traducción, no el texto: el idioma lo pone la vista
 * con `t()`. Aparte para poder probar los cortes de hora sin montar nada.
 *
 * Las franjas: madrugada y noche comparten saludo —de 19 a 4—, la mañana va de 5
 * a 11 y la tarde de 12 a 18.
 */
export function greetingKey(hour: number): string {
	if (hour >= 5 && hour < 12) return 'views.menu.greeting.morning';
	if (hour >= 12 && hour < 19) return 'views.menu.greeting.afternoon';
	return 'views.menu.greeting.evening';
}
