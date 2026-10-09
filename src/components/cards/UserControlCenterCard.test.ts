import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * La tarjeta del usuario en el centro de control no se puede tocar: muestra
 * quién sos, la hora y la fecha, y no responde a ningún clic. La excepción es
 * la fecha, que abre el calendario del mes (vasak-desktop#183): es el botón de
 * la librería, y la reacción al pasar el mouse es la suya, no de la tarjeta.
 *
 * Es la misma decisión que ya se tomó para los iconos de la bandeja —que no se
 * resalte lo que no se puede tocar—, y volvió a perderse acá: la tarjeta se
 * pintaba de otro color y se agrandaba al pasar el mouse, prometiendo un botón
 * que no existe. Se fija en un test porque es una regla de diseño, no un gusto
 * de una tarde.
 */

const COMPONENT = readFileSync(join(import.meta.dir, 'UserControlCenterCard.vue'), 'utf8');

describe('tarjeta del usuario', () => {
	test('no reacciona al pasar el mouse', () => {
		const reactions = [...COMPONENT.matchAll(/(?:group-)?hover:[a-z0-9:[\]/.-]+/g)].map(
			([className]) => className
		);

		expect(reactions).toEqual([]);
	});

	test('lo único que se toca es la fecha, y es el botón de la librería', () => {
		expect(COMPONENT.match(/<ActionButton\b/g)).toHaveLength(1);
		expect(COMPONENT).toMatch(
			/<ActionButton[\s\S]*?data-user-date[\s\S]*?@click="emit\('open-calendar'\)"/
		);
		expect(COMPONENT).not.toMatch(/<button\b/);
		expect(COMPONENT).not.toMatch(/@click="[^"]*"[^>]*data-user-card|data-user-card[^>]*@click/);
	});

	test('la foto del usuario es la de la librería', () => {
		// `Avatar` la recorta en círculo con `rounded-corner-full` —el radio que
		// eligió la persona— y cae a las iniciales si no hay foto o no carga.
		// La copia a mano dejaba la foto cuadrada dentro de un círculo que no
		// recortaba nada.
		expect(COMPONENT).toMatch(/<Avatar[^>]*size="xl"/);
		expect(COMPONENT).not.toContain('<img');
	});

	test('sin el fondo de la ventana encima de la ventana', () => {
		// `ui-bg` es el fondo de la ventana; una tarjeta va en la superficie.
		expect(COMPONENT).not.toMatch(/bg-ui-bg\b/);
		expect(COMPONENT).toContain('bg-ui-surface/70');
	});

	test('la fecha lleva mayúscula sólo en la primera letra', () => {
		// El `capitalize` de CSS la ponía en cada palabra: «Martes, 6 De
		// Octubre». La primera letra sale del formateo, con el idioma.
		expect(COMPONENT).not.toMatch(/class="[^"]*\bcapitalize\b/);
		expect(COMPONENT).toMatch(/capitalizeFirst\(\s*now\.toLocaleDateString/);
	});
});
