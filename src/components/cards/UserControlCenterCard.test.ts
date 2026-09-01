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

	test('la foto del usuario es un círculo', () => {
		const foto = COMPONENTE.slice(COMPONENTE.indexOf('<img'), COMPONENTE.indexOf('</div>'));

		// En la imagen, porque no hereda el redondeo del contenedor...
		expect(foto).toContain('rounded-full');
		// ...y recortada por la caja, para que un avatar que no sea cuadrado
		// tampoco se salga.
		expect(COMPONENTE).toContain('overflow-hidden rounded-full');
	});
});
