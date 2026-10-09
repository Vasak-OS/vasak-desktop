/**
 * El cálculo del desplazamiento de `MarqueeText`, aparte del componente para
 * poder probarlo sin medir una maqueta real (en el DOM de prueba `scrollWidth` y
 * `clientWidth` son cero).
 */

/**
 * Cuántos píxeles sobran: lo que el contenido mide de más que su caja. Cero si
 * entra. Se ignora un sobrante de un píxel para no animar por el redondeo del
 * subpíxel, que haría temblar un nombre que en realidad entra.
 */
export function marqueeShift(scrollWidth: number, clientWidth: number): number {
	const overflow = scrollWidth - clientWidth;
	return overflow > 1 ? overflow : 0;
}

/** `true` cuando el contenido no entra y hay que desplazarlo. */
export function marqueeOverflows(scrollWidth: number, clientWidth: number): boolean {
	return marqueeShift(scrollWidth, clientWidth) > 0;
}
