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

/**
 * El archivo sin lo que está comentado.
 *
 * Un import comentado no es un import: el componente no queda disponible y Vue
 * dibuja un elemento desconocido sin fallar. Se sacan los bloques y las líneas
 * que **empiezan** con `//`, no cualquier `//`, para que una URL adentro de una
 * cadena no se lleve media línea puesta.
 */
function sinComentarios(texto: string): string {
	return texto
		.replace(/\/\*[\s\S]*?\*\//g, '')
		.replace(/<!--[\s\S]*?-->/g, '')
		.split('\n')
		.filter((linea) => !/^\s*\/\//.test(linea))
		.join('\n');
}

describe('no hay controles propios', () => {
	test('ningún interruptor escrito acá', () => {
		// El de la librería lleva `role="switch"`, `aria-checked`, nombre
		// obligatorio y anillo de foco. Uno nuevo empieza sin nada de eso.
		expect(conteniendo(/role="switch"/)).toEqual([]);
	});

	test('ningún campo de texto escrito acá', () => {
		// Eran tres: el del menú —un `<input>` suelto con una directiva propia
		// para el foco—, la clave del Wi-Fi —que no tenía nombre accesible: un
		// marcador no es un nombre, se borra al escribir— y el filtro del menú
		// de conexiones.
		//
		// Había una excepción nombrada, `SearchView`: la búsqueda global, un
		// campo sin borde adentro de su propia caja, que imponerle el aspecto
		// compartido volvía otra cosa. Esa ventana se fue a `vasak-prism`, así
		// que la excepción se va con ella y no quedan campos escritos acá.
		expect(conteniendo(/<input(?![^>]*type="range")/)).toEqual([]);
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

	/**
	 * Los cuatro controles que la librería pasó a tener.
	 *
	 * Los cuatro tests eran lo mismo —el archivo importa de la librería y usa
	 * tal componente— con otro par de valores, así que van en una tabla. El
	 * `nombre` va en la tabla y no en un `for` porque sin él un fallo sale como
	 * «esperaba esta cadena y llegó esta otra» y no dice de cuál de los cuatro
	 * componentes habla: los cuatro repiten las mismas dos aserciones.
	 *
	 * El nombre del componente va en su propia columna y no en el `nombre` del
	 * caso a propósito. Uno es lo que hay que encontrar en el archivo; el otro es
	 * la manera de nombrarlo en prosa. Que sean dos columnas y no una es lo que
	 * deja que un fallo diga «el interruptor del Bluetooth» sin que el nombre
	 * técnico y el nombre de la prueba sean la misma cosa.
	 *
	 * Los nombres se dejan tal cual estaban, incluido el «y» del último: son los
	 * que ya identifican estos casos en el informe, y renombrarlos obliga a
	 * buscar en dos lugares cuando algo falle.
	 */
	const CONTROLES = [
		{
			nombre: 'el interruptor del Bluetooth',
			archivo: 'components/areas/bluetooth/BluetoothControlArea.vue',
			control: 'SwitchToggle',
		},
		{
			nombre: 'el botón que alterna la red',
			archivo: 'components/controls/NetworkControl.vue',
			control: 'ToggleControl',
		},
		{
			nombre: 'el deslizador del volumen',
			archivo: 'components/controls/VolumeControl.vue',
			control: 'SliderControl',
		},
		{
			nombre: 'y el buscador del menú',
			archivo: 'views/MenuView.vue',
			control: 'SearchField',
		},
	];

	test.each(CONTROLES)('$nombre', ({ archivo, control }) => {
		const fuente = leer(archivo);
		expect(fuente).toMatch(DE_LA_LIBRERIA);
		expect(fuente).toContain(control);
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

/**
 * El botón de acción, que era la copia más parecida de todas.
 *
 * `ActionButton` de acá y el de la librería eran **el mismo archivo** salvo la
 * sangría y una clase — y ahí la de la librería sabía más: usa
 * `transition-[background-color,opacity]` donde la copia usaba
 * `transition-all`. Animar «todas» incluye las propiedades de maquetado, que es
 * justo lo que cuesta caro en WebKitGTK y lo que midió el perfil de
 * `vasak-resonance`; acá pasaba en cada hover de cada botón.
 *
 * No hay guardia contra `transition-all` porque quedan ocho archivos más que lo
 * usan —transiciones de lista, la bandeja, el filtro del menú— y revisarlos es
 * un barrido aparte: una guardia que naciera con ocho excepciones no guardaría
 * nada.
 */
describe('el botón de acción', () => {
	test('ya no hay copia propia', () => {
		expect(fuentes.filter(({ ruta }) => ruta.endsWith('buttons/ActionButton.vue'))).toEqual([]);
	});

	test('y los dos que lo usan lo piden a la librería', () => {
		// Si alguno lo usa sin importarlo, Vue dibuja un elemento desconocido y
		// no falla: el botón no está y nadie se entera.
		const culpables = fuentes
			.filter(({ texto }) => /<ActionButton\b/.test(sinComentarios(texto)))
			.filter(
				({ texto }) =>
					!/import \{[^}]*\bActionButton\b[^}]*\} from '@vasakgroup\/vue-libvasak'/.test(
						sinComentarios(texto)
					)
			)
			.map(({ ruta }) => ruta);

		expect(culpables).toEqual([]);
	});
});
