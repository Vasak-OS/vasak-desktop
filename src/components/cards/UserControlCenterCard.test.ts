import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * La tarjeta del usuario en el centro de control no se puede tocar: muestra
 * quién sos, la hora y la fecha, y no responde a ningún clic.
 *
 * Es la misma decisión que ya se tomó para los iconos de la bandeja —que no se
 * resalte lo que no se puede tocar—, y volvió a perderse acá: la tarjeta se
 * pintaba de otro color y se agrandaba al pasar el mouse, prometiendo un botón
 * que no existe. Se fija en un test porque es una regla de diseño, no un gusto
 * de una tarde.
 */

const COMPONENTE = readFileSync(join(import.meta.dir, 'UserControlCenterCard.vue'), 'utf8');

describe('tarjeta del usuario', () => {
	test('no reacciona al pasar el mouse', () => {
		const reacciones = [...COMPONENTE.matchAll(/(?:group-)?hover:[a-z0-9:[\]/.-]+/g)].map(
			([clase]) => clase
		);

		expect(reacciones).toEqual([]);
	});

	test('la foto del usuario es la de la librería', () => {
		// `Avatar` la recorta en círculo con `rounded-corner-full` —el radio que
		// eligió la persona— y cae a las iniciales si no hay foto o no carga.
		// La copia a mano dejaba la foto cuadrada dentro de un círculo que no
		// recortaba nada.
		expect(COMPONENTE).toMatch(/<Avatar[^>]*size="xl"/);
		expect(COMPONENTE).not.toContain('<img');
	});

	test('sin el fondo de la ventana encima de la ventana', () => {
		// `ui-bg` es el fondo de la ventana; una tarjeta va en la superficie.
		expect(COMPONENTE).not.toMatch(/bg-ui-bg\b/);
		expect(COMPONENTE).toContain('bg-ui-surface/70');
	});
});
