/**
 * El icono que se mide con su caja, y la trampa que lo tenía cortado.
 *
 * `WeatherIcon` dibujaba `<ThemeIcon :size="64" class="h-full w-full" />`. Las
 * dos cosas juntas no se llevan: con un número, `ThemeIcon` escribe el alto y
 * el ancho **en línea**, y una regla en línea le gana a cualquier clase. O sea
 * que el `h-full w-full` no hacía nada y el dibujo se quedaba clavado en 64
 * píxeles adentro de una caja que podía ser mucho más chica —22cqmin en el
 * widget, `h-5` en la bandeja—, así que se salía y se veía cortado.
 *
 * Desde `@vasakgroup/vue-libvasak` 1.5.0, `size="auto"` hace que no escriba
 * estilo y el tamaño salga de la caja. Que eso funciona está probado **montado**
 * allá, en `tests/el-icono-que-se-mide-con-su-caja.test.ts` de la librería; acá
 * se vigila que esta aplicación lo pida así, que es la parte que le toca.
 *
 * Se mira el texto y no se monta porque este repositorio todavía no tiene con
 * qué montar, igual que `controles-de-la-libreria.test.ts`.
 *
 * La segunda prueba es la que importa a futuro: no vigila al clima, vigila la
 * **trampa**. Un número junto a una clase de tamaño es siempre el mismo error,
 * y no falla ni avisa — se ve recién cuando la caja es más chica que el número.
 */

import { describe, expect, test } from 'bun:test';
import { Glob } from 'bun';
import { fileURLToPath } from 'node:url';

const FUENTE = fileURLToPath(new URL('../src/', import.meta.url));

const fuentes = await Promise.all(
	[...new Glob('**/*.vue').scanSync(FUENTE)].map(async (ruta) => ({
		ruta,
		texto: await Bun.file(FUENTE + ruta).text(),
	}))
);

const leer = (ruta: string) => fuentes.find((f) => f.ruta === ruta)?.texto ?? '';

/**
 * El borde de un atributo.
 *
 * Sin él, `size="auto"` también encuentra a `data-size="auto"` y `class="…"` a
 * `data-class="…"`: la prueba pasaría con el icono **sin** la propiedad puesta,
 * que es justamente la regresión que vigila. El nombre de un atributo empieza
 * después de un espacio, así que alcanza con exigirlo.
 */
const BORDE = String.raw`[^>]*\s`;

/** Cada etiqueta `<ThemeIcon …>` del repositorio, con el archivo donde vive. */
function cadaIcono(): { ruta: string; etiqueta: string }[] {
	const salida: { ruta: string; etiqueta: string }[] = [];
	for (const { ruta, texto } of fuentes) {
		for (const m of texto.matchAll(/<ThemeIcon\b[^>]*?\/?>/gs)) {
			salida.push({ ruta, etiqueta: m[0] });
		}
	}
	return salida;
}

describe('el icono del clima', () => {
	test('hay algo que mirar', () => {
		// Sin esto, las de abajo pasan sobre una lista vacía el día que el
		// patrón deje de encontrar archivos.
		expect(leer('components/icon/WeatherIcon.vue')).toContain('ThemeIcon');
		expect(cadaIcono().length).toBeGreaterThan(10);
	});

	test('pide el tamaño a su caja y no en píxeles', () => {
		const clima = leer('components/icon/WeatherIcon.vue');

		expect(clima).toMatch(new RegExp(`<ThemeIcon${BORDE}size="auto"`, 's'));
		// El número que estaba: si vuelve, vuelve el recorte. `v-bind:size`
		// también, que es lo mismo escrito largo.
		expect(clima).not.toMatch(new RegExp(`<ThemeIcon${BORDE}(?:v-bind)?:size="\\d`, 's'));
	});

	test('y la caja sigue diciendo cuánto mide', () => {
		// `size="auto"` sin clases deja al icono en su tamaño natural, que es
		// otro problema con la misma cara. Las dos mitades van juntas.
		expect(leer('components/icon/WeatherIcon.vue')).toMatch(
			new RegExp(`<ThemeIcon${BORDE}class="[^"]*\\bh-full\\b[^"]*\\bw-full\\b`, 's')
		);
	});
});

describe('la trampa, para todo el repositorio', () => {
	test('ningún icono mezcla un tamaño en píxeles con clases de tamaño', () => {
		// Es el mismo error del clima, escrito en cualquier otro lado. El estilo
		// en línea gana y la clase queda de adorno: nada falla, y se ve mal
		// recién cuando la caja es más chica que el número.
		const deTamano = /\b(h-full|w-full|size-full|h-\[|w-\[|size-\d)/;
		const enPixeles = new RegExp(String.raw`\s(?:v-bind)?:size="\d`);
		const culpables = cadaIcono()
			.filter(({ etiqueta }) => enPixeles.test(etiqueta) && deTamano.test(etiqueta))
			.map(({ ruta }) => ruta);

		expect(culpables).toEqual([]);
	});
});
