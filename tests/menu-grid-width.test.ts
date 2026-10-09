/**
 * Las celdas del menú confinan el nombre a su ancho (vasak-desktop#219 de nuevo).
 *
 * El `MarqueeText` recorta/desplaza con `w-full` + `overflow-hidden`, pero eso
 * sólo acota si su contenedor tiene ancho acotado. `DropdownMenuItem` (vue-libvasak)
 * mete el slot en un `<span>` que, con la columna `items-center`, queda del ancho
 * del CONTENIDO: un nombre largo se desbordaba y se pisaba con la celda vecina
 * (se vio en la grilla real, no en el banco del componente aislado). La celda
 * fuerza ese `<span>` a ancho completo con la variante `[&>span]:w-full`; sin eso
 * el nombre vuelve a romper la grilla. Esta guardia lo fija en el fuente.
 */
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = join(import.meta.dir, '..');
const read = (p: string) => readFileSync(join(ROOT, p), 'utf8');

describe('las celdas del menú no dejan que el nombre rompa la grilla', () => {
	test('AppTile (grilla y mosaicos) fuerza el ancho del slot', () => {
		expect(read('src/components/areas/menu/AppTile.vue')).toContain('[&>span]:w-full');
	});

	test('FavoritesArea fuerza el ancho del slot', () => {
		expect(read('src/components/areas/menu/FavoritesArea.vue')).toContain('[&>span]:w-full');
	});
});
