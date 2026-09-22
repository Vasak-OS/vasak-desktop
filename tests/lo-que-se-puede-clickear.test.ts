/**
 * Lo que responde al clic, responde también al teclado.
 *
 * Los iconos del panel eran `<div>` con `@click`: sirven con el mouse y no
 * existen para quien navega con Tab. Un `<div>` no recibe foco, no se activa
 * con Enter ni con la barra, y el lector de pantalla no lo anuncia como algo
 * que se pueda usar. Nada de eso falla ni avisa —el icono se ve igual—, que es
 * por qué sobrevivió tanto.
 *
 * Y un botón cuyo contenido es un dibujo no tiene texto: sin nombre accesible
 * se anuncia como «botón», a secas. Por eso no alcanza con cambiar la etiqueta.
 *
 * Se mira el texto y no se monta porque este repositorio todavía no tiene con
 * qué montar, igual que `controles-de-la-libreria.test.ts`. La lista de abajo
 * es exhaustiva a propósito: si vuelve a aparecer un `<div @click>`, aunque sea
 * uno nuevo, esta prueba lo nombra.
 */

import { describe, expect, test } from 'bun:test';
import { Glob } from 'bun';

const raiz = new URL('../src/', import.meta.url).pathname;

const fuentes = await Promise.all(
	[...new Glob('**/*.vue').scanSync(raiz)].map(async (ruta) => ({
		ruta,
		texto: await Bun.file(raiz + ruta).text(),
	}))
);

const leer = (ruta: string) => fuentes.find((f) => f.ruta === ruta)?.texto ?? '';

/**
 * Las filas enteras que se pueden tocar.
 *
 * Son otra discusión: una tarjeta que se abre al hacer clic en cualquier punto
 * no se arregla poniéndole `<button>` alrededor —adentro tienen sus propios
 * botones, y un botón dentro de otro no es HTML válido—. Queda anotado en
 * https://github.com/Vasak-OS/vasak-desktop/issues/102 y no se toca acá.
 */
const FILAS_ENTERAS = new Set([
	'components/cards/DeviceCard.vue',
	'components/cards/ListCard.vue',
	'components/cards/NotificationCard.vue',
	'components/cards/NotificationGroupCard.vue',
	'components/controls/AudioDeviceSelector.vue',
	'views/apps/SearchView.vue',
]);

/** Etiquetas nativas que no hacen nada por sí solas al recibir un clic. */
const SORDAS = /<(div|span|img|li|p|a)\b((?:[^<>]|"[^"]*"|'[^']*')*?)>/g;

function sordasQueEscuchanElClic(texto: string): number {
	let cuantas = 0;
	for (const [, , atributos] of texto.matchAll(SORDAS)) {
		if (!/(@click|v-on:click)\b/.test(atributos)) continue;
		if (/role="button"/.test(atributos)) continue;
		cuantas += 1;
	}
	return cuantas;
}

describe('nada que se pueda clickear queda fuera del teclado', () => {
	test('ningún elemento sordo escucha el clic, salvo las filas enteras', () => {
		const culpables = fuentes
			.filter(({ ruta }) => !FILAS_ENTERAS.has(ruta))
			.filter(({ texto }) => sordasQueEscuchanElClic(texto) > 0)
			.map(({ ruta }) => ruta);

		expect(culpables).toEqual([]);
	});

	test('y la lista de excepciones no tiene nombres de más', () => {
		// Una excepción que ya no hace falta es una puerta abierta: el día que
		// alguien vuelva a poner un `@click` en ese archivo, nadie se entera.
		const sobrantes = [...FILAS_ENTERAS].filter(
			(ruta) => sordasQueEscuchanElClic(leer(ruta)) === 0
		);

		expect(sobrantes).toEqual([]);
	});
});

describe('los botones del panel se anuncian con nombre', () => {
	test('el icono de la bandeja se llama como su `alt`, y si no, como su tooltip', () => {
		const componente = leer('components/buttons/TrayIconButton.vue');

		expect(componente).toContain("props.alt || props.tooltip || undefined");
		expect(componente).toContain("'aria-label': nombreAccesible");
	});

	test('el que sólo informa no finge ser un botón', () => {
		// La batería, Bloq Mayús y el micrófono silenciado no hacen nada al
		// tocarlos: anunciarlos como botones promete un clic que no existe.
		const componente = leer('components/buttons/TrayIconButton.vue');

		expect(componente).toContain("interactive ? 'button' : 'div'");
	});

	test('los otros tres del panel llevan su nombre puesto', () => {
		expect(leer('components/buttons/TrayIconPrivacy.vue')).toContain(':aria-label="detalle"');
		expect(leer('components/buttons/WindowPanelButton.vue')).toContain(':aria-label="title"');
		expect(leer('views/PanelView.vue')).toContain(
			':aria-label="t(\'views.panel.notificationsAlt\')"'
		);
	});
});
