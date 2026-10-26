/**
 * El desplazamiento y el tope de la píldora de ventanas abiertas
 * (vasak-desktop#151, #161, #213, tope de 3 iconos en #251).
 *
 * La píldora no se pliega nunca —son sólo iconos—, así que lo que no entra se
 * desplaza adentro, sin barra visible (`.windows-pill` esconde la del navegador).
 * Cuánto se ve antes de desplazar, y sobre qué eje, depende de de qué lado va el
 * panel y de la densidad. La lógica vive acá, aparte de la vista, para poder
 * probarla sin montar Vue.
 */
import type { PanelLayout } from '@/tools/panel-appearance';

/** Cuántos iconos se ven antes de que el resto se desplace adentro. */
export const WINDOWS_PILL_VISIBLE = 3;

/**
 * Las medidas reales de un botón de ventana y de la píldora, en `rem`. No son
 * claves mágicas: son las que ya usan el botón y la píldora en el marcado.
 *
 * - El botón es fijo: `shrink-0 min-w-8 min-h-8`, 2rem, el blanco táctil mínimo
 *   (ver `WindowPanelButton.vue`). No se encoge, así que medirlo da siempre esto.
 * - La fila los separa con `gap-0.5`, 0.125rem.
 * - La píldora deja 0.25rem de relleno a cada lado (`px-1` en horizontal,
 *   `py-1` de costado) y lleva `border` (1px = 0.0625rem) a cada lado. Con
 *   `box-sizing: border-box`, el tope mide el total, así que relleno y borde
 *   cuentan o el tercer icono queda cortado.
 */
const BUTTON_REM = 2;
const GAP_REM = 0.125;
const PADDING_SIDE_REM = 0.25;
const BORDER_SIDE_REM = 0.0625;

/**
 * El tope de la píldora, en `rem`, para `WINDOWS_PILL_VISIBLE` botones: tres
 * botones, las dos separaciones del medio, el relleno y el borde de los dos
 * lados. `3·2 + 2·0.125 + 2·0.25 + 2·0.0625 = 6.875rem`. Reemplaza al viejo
 * `max-w-[90vw]` del compacto, que dejaba la píldora cortada sin poder desplazar.
 */
export const WINDOWS_PILL_CAP_REM =
	WINDOWS_PILL_VISIBLE * BUTTON_REM +
	(WINDOWS_PILL_VISIBLE - 1) * GAP_REM +
	2 * PADDING_SIDE_REM +
	2 * BORDER_SIDE_REM;

/**
 * Las clases de desplazamiento de la píldora, según el lado y la densidad.
 *
 * - De costado (vertical): columna que toma el alto que sobra (`flex-1`) y
 *   desplaza hacia abajo (`overflow-y-auto`); el tope de alto lo pone el
 *   `:style`, no una clase.
 * - Horizontal **compacto**: fila que reserva su ancho (`shrink-0`) y desplaza a
 *   lo largo (`overflow-x-auto`); el tope de ancho lo pone el `:style`.
 * - Horizontal **distribuido/completo** (#161): cede a las vecinas (`min-w-0`,
 *   sin `shrink-0`) hasta `max-w-[20rem]` y desplaza adentro. Sin cambios.
 *
 * El tope absoluto de 3 iconos va aparte, en `windowsPillCapStyle`, para que el
 * valor salga de las medidas reales y no de una clase estática del JIT.
 */
export function windowsPillClasses(vertical: boolean, layout: PanelLayout): string[] {
	if (vertical) {
		return ['min-h-0', 'flex-1', 'py-1', 'overflow-y-auto', 'overflow-x-hidden'];
	}
	if (layout === 'compact') {
		return ['px-1', 'overflow-x-auto', 'overflow-y-hidden', 'shrink-0'];
	}
	return ['px-1', 'overflow-x-auto', 'overflow-y-hidden', 'max-w-[20rem]'];
}

/**
 * El tope de la píldora como `style` en línea: de alto de costado, de ancho en
 * horizontal compacto, y nada en horizontal distribuido/completo (ahí el tope lo
 * pone `max-w-[20rem]`). Va por `:style` y no por una clase para que el valor
 * salga de `WINDOWS_PILL_CAP_REM` —medido— sin depender de que el JIT de Tailwind
 * haya generado una clase con ese número.
 */
export function windowsPillCapStyle(
	vertical: boolean,
	layout: PanelLayout
): Record<string, string> {
	const cap = `${WINDOWS_PILL_CAP_REM}rem`;
	if (vertical) return { maxHeight: cap };
	if (layout === 'compact') return { maxWidth: cap };
	return {};
}

/**
 * Desplaza la píldora con la rueda, sobre el eje que la píldora desplaza.
 *
 * En horizontal lo que no entra se desplaza a lo largo (`overflow-x-auto`), pero
 * una rueda vertical sobre un desbordamiento horizontal no hace nada por omisión
 * en WebKitGTK (el mismo caso que las pestañas en `TabBar.vue` de vue-libvasak):
 * sin un mouse con rueda horizontal, las ventanas de más quedaban inalcanzables.
 * Acá la rueda vertical mueve el carril en su eje real. Devuelve si de verdad se
 * movió, para que la vista llame a `preventDefault` sólo entonces —en el tope, o
 * sin desbordamiento, el desplazamiento sigue hacia lo que haya debajo.
 */
export function scrollWindowsPillOnWheel(
	node: { scrollLeft: number; scrollTop: number },
	vertical: boolean,
	deltaY: number,
	deltaX: number
): boolean {
	const amount = deltaY || deltaX || 0;
	if (vertical) {
		const before = node.scrollTop;
		node.scrollTop += amount;
		return node.scrollTop !== before;
	}
	const before = node.scrollLeft;
	node.scrollLeft += amount;
	return node.scrollLeft !== before;
}
