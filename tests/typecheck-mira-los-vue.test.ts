/**
 * Que el chequeo de tipos mire los componentes.
 *
 * `bunx --bun vue-tsc` no comprueba ningún `.vue`. Bajo el runtime de Bun,
 * `vue-tsc` no encuentra el gancho que le deja leer los archivos de un solo
 * componente y degrada a `tsc` a secas: mira los `.ts` y salta los `.vue`. No
 * avisa por ningún lado y termina en cero, así que el `build` venía dando el
 * visto bueno sin haber abierto casi nada del código.
 *
 * Se comprobó metiendo un `const x: number = 'texto'` en un componente: con
 * `--bun` no sale nada; sin él, el error salta.
 */

import { describe, expect, test } from 'bun:test';

const manifiesto = (await Bun.file(new URL('../package.json', import.meta.url)).json()) as {
	scripts: Record<string, string>;
};

describe('el chequeo de tipos', () => {
	test('no corre bajo el runtime de Bun', () => {
		// Se miran todos los scripts y no sólo `build`: la trampa es la misma
		// desde donde sea que se llame.
		const ciegos = Object.entries(manifiesto.scripts)
			.filter(([, orden]) => /--bun[^&|]*vue-tsc/.test(orden))
			.map(([nombre]) => nombre);

		expect(ciegos).toEqual([]);
	});

	test('y sigue estando', () => {
		// Sacar el `--bun` quitando la llamada entera también «arregla» esta
		// prueba, y deja la aplicación sin comprobar los tipos.
		const llama = Object.values(manifiesto.scripts).some((orden) =>
			orden.includes('vue-tsc --noEmit')
		);

		expect(llama).toBe(true);
	});
});
