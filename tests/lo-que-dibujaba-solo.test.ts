/**
 * La última copia del composable del icono, y la más grande.
 *
 * Trescientas sesenta y siete líneas y cuarenta y cinco llamadas en cuarenta y
 * un archivos. No era una copia cualquiera: tenía un planificador que la
 * librería no tenía —rebote del aviso de cambio de tema, cancelación del ciclo a
 * medio correr, prioridad por visibilidad con un `IntersectionObserver` y tandas
 * de diez— y lo tenía **sin una sola prueba**.
 *
 * Existía por una razón buena: `MenuArea` dibuja la lista entera de aplicaciones
 * instaladas, entre sesenta y ciento cincuenta iconos con nombres todos
 * distintos, así que la memoria de la librería no ayudaba y resolverlos de golpe
 * era una llamada al backend por aplicación en el proceso que dibuja el panel.
 *
 * Por eso esto no se pudo hacer de una: el planificador subió a la librería
 * (vue-libvasak#58, desde la 1.3.0) y cuatro componentes que pedían el icono
 * como ruta ya resuelta pasaron a pedirlo por nombre (#59, desde la 1.4.0).
 * Recién ahí se pudo borrar la copia sin perder nada.
 *
 * Ver Vasak-OS/vue-libvasak#54 y #57.
 */

import { describe, expect, test } from 'bun:test';
import { Glob } from 'bun';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

// `fileURLToPath` y no `.pathname`: éste deja los caracteres codificados tal
// como están, así que un checkout en una ruta con un espacio llega con `%20` y
// `scanSync` no encuentra nada.
const FUENTE = fileURLToPath(new URL('../src/', import.meta.url));

// Las pruebas que viven al lado del componente quedan afuera: **hablan** del
// código, así que nombran lo que vigilan. La de `TrayIconPrivacy` cuenta en un
// comentario que los nombres ya no salen de `useSymbol('…')`, y con eso sola
// hacía fallar a la guardia.
const fuentes = [...new Glob('**/*.{vue,ts}').scanSync(FUENTE)].filter(
	(ruta) => !ruta.endsWith('.test.ts')
);

/**
 * Los dos que siguen resolviendo a mano, y por qué.
 *
 * `tools/file.controller.ts` estaba acá y se fue. Resolvía el icono de cada
 * archivo y lo guardaba **en el dato**, con una rama para pasarlo por
 * `convertFileSrc` cuando lo que volvía era una ruta absoluta. Esa rama nunca
 * corrió: `getIconSource` devuelve siempre un `data:` —lo arma el propio
 * complemento, en `guest-js`— o la cadena vacía. Sin ella no quedaba nada que
 * `ThemeIcon` no hiciera, así que el widget pasó a recibir el **nombre** del
 * icono y a dibujarlo con el componente, que es lo que lo hace seguir al tema.
 *
 * - `tools/composables/useMusicPlayer.ts`: la tapa de respaldo del reproductor.
 *   Ahí hace falta una **ruta** y no un nombre, porque `imgSrc` es la carátula
 *   que manda el reproductor y se dibuja con un `img` común: el respaldo tiene
 *   que ser algo que ese mismo `img` pueda mostrar.
 * - `main.ts`: el menú contextual del escritorio no dibuja con Vue, pide una
 *   **función** que resuelva el nombre a una ruta porque lo pinta el
 *   complemento fuera de esta ventana.
 */
const EXCEPCIONES = new Set(['tools/composables/useMusicPlayer.ts', 'main.ts']);

describe('el composable de iconos propio', () => {
	test('hay algo que mirar', () => {
		// Sin esto las de abajo pasan sobre una lista vacía, que es en lo que
		// quedan si el patrón deja de encontrar archivos. Una guardia que se
		// apaga sola dice que sí.
		expect(fuentes).toContain('views/PanelView.vue');
		expect(fuentes.length).toBeGreaterThan(80);
	});

	test('ya no está', () => {
		expect(fuentes.filter((ruta) => ruta.includes('useReactiveIcon'))).toEqual([]);
	});

	test('y nadie lo llama', async () => {
		const culpables: string[] = [];
		for (const ruta of fuentes) {
			const texto = await Bun.file(join(FUENTE, ruta)).text();
			if (/\buse(Reactive)?(Icon|Symbol)s?\s*\(/.test(texto)) culpables.push(ruta);
		}

		expect(culpables).toEqual([]);
	});
});

describe('quién resuelve iconos a mano', () => {
	test('sólo los dos que no pueden hacerlo de otra forma', async () => {
		// Lo que cuenta es **importar** el complemento o escuchar el evento del
		// cambio de tema, no nombrarlos: un archivo puede recibir el resolvedor
		// como parámetro y eso no es una copia.
		const aMano: string[] = [];
		for (const ruta of fuentes) {
			const texto = await Bun.file(join(FUENTE, ruta)).text();
			const importa = /from '@vasakgroup\/plugin-vicons'/.test(texto);
			const escucha = /listen\(\s*'vicons:theme-changed'/.test(texto);
			if (importa || escucha) aMano.push(ruta);
		}

		expect(aMano.sort()).toEqual([...EXCEPCIONES].sort());
	});

	test('y nadie arma su propio planificador de recarga', async () => {
		// El que había acá subió a la librería. Que vuelva a aparecer uno en una
		// aplicación es la misma historia otra vez, con otro nombre.
		const culpables: string[] = [];
		for (const ruta of fuentes) {
			const texto = await Bun.file(join(FUENTE, ruta)).text();
			if (/IntersectionObserver/.test(texto) && /theme|tema/i.test(texto)) culpables.push(ruta);
		}

		expect(culpables).toEqual([]);
	});
});
