/**
 * El escritorio no se dibuja sus propios controles.
 *
 * Tenía cuatro: el interruptor, el botón que alterna, el deslizador y el campo
 * del buscador del menú. Los cuatro existían también en la librería, y en los
 * cuatro la copia de acá **sabía más**: nombre accesible obligatorio,
 * `aria-pressed` con el criterio de cuándo no ponerlo, `aria-valuetext`, y un
 * anillo de foco que se agregaba por fuera con una clase suelta.
 *
 * Eso subió a la librería y las copias se fueron. Lo que se vigila acá es que
 * no vuelvan: nada de esto falla ni avisa —simplemente se ve distinto y se oye
 * peor—, que es exactamente como se separaron la primera vez.
 *
 * Se mira el texto y no se monta porque este repositorio todavía no tiene con
 * qué montar, igual que `emits-declarados.test.ts`. Lo que las copias hacían y
 * ahora hace la librería está probado allá, montado.
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

function conteniendo(patron: RegExp): string[] {
	return fuentes.filter(({ texto }) => patron.test(texto)).map(({ ruta }) => ruta);
}

const leer = (ruta: string) => fuentes.find((f) => f.ruta === ruta)?.texto ?? '';

describe('no hay controles propios', () => {
	test('ningún interruptor escrito acá', () => {
		// El de la librería lleva `role="switch"`, `aria-checked`, nombre
		// obligatorio y anillo de foco. Uno nuevo empieza sin nada de eso.
		expect(conteniendo(/role="switch"/)).toEqual([]);
	});

	test('ningún campo de texto escrito acá, salvo la búsqueda global', () => {
		// Eran tres: el del menú —un `<input>` suelto con una directiva propia
		// para el foco—, la clave del Wi-Fi —que no tenía nombre accesible: un
		// marcador no es un nombre, se borra al escribir— y el filtro del menú
		// de conexiones.
		//
		// `SearchView` se queda con el suyo **a propósito**: es la búsqueda
		// global, un campo sin borde y de veinte píxeles adentro de su propia
		// caja, no un campo de formulario. Imponerle el aspecto compartido sería
		// volverla otra cosa. Es la única excepción, y está nombrada acá para
		// que agregar una segunda tenga que pasar por esta prueba.
		const EXCEPCION = 'views/apps/SearchView.vue';
		expect(conteniendo(/<input(?![^>]*type="range")/)).toEqual([EXCEPCION]);
	});

	test('ni una directiva de foco propia', () => {
		// La había: `setTimeout(() => el.focus(), 50)` sobre el buscador del
		// menú, porque el atributo `autofocus` del HTML no sirve para un campo
		// que aparece después de cargar el documento. Ahora lo hace el campo.
		expect(conteniendo(/const vFocus\b/)).toEqual([]);
	});
});

describe('los controles salen de la librería', () => {
	const DE_LA_LIBRERIA = /from '@vasakgroup\/vue-libvasak'/;

	test('el interruptor del Bluetooth', () => {
		const fuente = leer('components/areas/bluetooth/BluetoothControlArea.vue');
		expect(fuente).toMatch(DE_LA_LIBRERIA);
		expect(fuente).toContain('SwitchToggle');
	});

	test('el botón que alterna la red', () => {
		const fuente = leer('components/controls/NetworkControl.vue');
		expect(fuente).toMatch(DE_LA_LIBRERIA);
		expect(fuente).toContain('ToggleControl');
	});

	test('el deslizador del volumen', () => {
		const fuente = leer('components/controls/VolumeControl.vue');
		expect(fuente).toMatch(DE_LA_LIBRERIA);
		expect(fuente).toContain('SliderControl');
	});

	test('y el buscador del menú', () => {
		const fuente = leer('views/MenuView.vue');
		expect(fuente).toMatch(DE_LA_LIBRERIA);
		expect(fuente).toContain('SearchField');
	});
});

describe('el interruptor deja de traer sus colores', () => {
	test('nadie le pasa clases de estado', () => {
		// El área del Bluetooth le pasaba `active-class`, `inactive-class` y un
		// `custom-class` con el anillo de foco. Los colores son justo lo que
		// hacía que este interruptor se viera distinto del de las demás
		// ventanas, y el anillo ahora viene adentro.
		//
		// El patrón mira sólo la etiqueta del interruptor: `enter-active-class`
		// y `leave-active-class` son de las transiciones de Vue y las usa medio
		// repositorio.
		expect(conteniendo(/<SwitchToggle\b[^>]*\b(?:active|inactive|custom)-class=/s)).toEqual([]);
	});
});

describe('y el guardia encuentra lo que busca', () => {
	test('sus patrones reconocen lo que vendrían a atajar', () => {
		// Sin esto, un patrón que dejara de reconocer la forma que busca dejaría
		// las pruebas de arriba en verde sin haber mirado nada.
		const plantado =
			'<b role="switch"> <input class="x"> const vFocus = {} <SwitchToggle active-class="y">';
		for (const patron of [
			/role="switch"/,
			/<input(?![^>]*type="range")/,
			/const vFocus\b/,
			/<SwitchToggle\b[^>]*\b(?:active|inactive|custom)-class=/s,
		]) {
			expect(patron.test(plantado)).toBe(true);
		}
	});

	test('y no confunden el rango del deslizador con un campo', () => {
		expect(/<input(?![^>]*type="range")/.test('<input type="range" min="0">')).toBe(false);
	});

	test('ni la transición de Vue con las clases del interruptor', () => {
		// `enter-active-class` y `leave-active-class` son de `<Transition>` y
		// las usa medio repositorio: un patrón que las confundiera dejaría esa
		// prueba en rojo para siempre y terminaría borrada.
		const transicion = '<Transition enter-active-class="x" leave-active-class="y">';
		expect(/<SwitchToggle\b[^>]*\b(?:active|inactive|custom)-class=/s.test(transicion)).toBe(false);
	});
});
