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

/** Etiquetas nativas que no hacen nada por sí solas al recibir un clic. */
const SORDAS = /<(div|span|img|li|p|a)\b((?:[^<>]|"[^"]*"|'[^']*')*?)>/g;

function sordasQueEscuchanElClic(texto: string): number {
	let cuantas = 0;
	for (const [, , atributos] of texto.matchAll(SORDAS)) {
		if (!/(@click|v-on:click)\b/.test(atributos)) continue;
		// El papel puesto a mano y el atado a una condición valen los dos. Varias
		// de estas filas **sólo a veces** hacen algo —`clickable`, o que la
		// notificación traiga acción por omisión—, y ahí el papel tiene que
		// aparecer y desaparecer con eso: anunciar un botón que no hace nada es
		// el mismo problema al revés. Cuando no lo es, el manejador corta
		// primero, así que el `@click` que queda no hace nada.
		if (/(?:^|\s):?role="[^"]*button/.test(atributos)) continue;
		cuantas += 1;
	}
	return cuantas;
}

describe('nada que se pueda clickear queda fuera del teclado', () => {
	test('ningún elemento sordo escucha el clic — ya sin excepciones', () => {
		// Hubo cinco: las filas enteras de las tarjetas y el selector de audio.
		// Eran otra discusión porque varias tienen **sus propios botones
		// adentro** y un botón dentro de otro no es HTML válido. Se resolvieron
		// caso por caso y la lista quedó vacía, así que acá ya no hay salvedad
		// que hacer.
		const culpables = fuentes
			.filter(({ texto }) => sordasQueEscuchanElClic(texto) > 0)
			.map(({ ruta }) => ruta);

		expect(culpables).toEqual([]);
	});
});

/**
 * Que no sea un `<div>` sordo no alcanza: `role="button"` sin `tabindex` no
 * recibe foco, y con foco pero sin manejadores de tecla no se activa. Las tres
 * cosas van juntas o no sirve ninguna — y como nada de esto falla ni avisa,
 * sólo se nota probándolo con el teclado, que es lo que nadie hace.
 */
describe('las filas que se abren enteras', () => {
	const CON_BOTONES_ADENTRO = [
		'components/cards/DeviceCard.vue',
		'components/cards/ListCard.vue',
		'components/cards/NotificationCard.vue',
		'components/cards/NotificationGroupCard.vue',
	];

	test.each(CON_BOTONES_ADENTRO)('%s se enfoca y se activa con el teclado', (ruta) => {
		const texto = leer(ruta);

		expect(texto).toMatch(/(?::role="|\srole=")/);
		expect(texto).toMatch(/(?::tabindex="|\stabindex=")/);
		expect(texto).toContain('@keydown.enter.prevent');
		// `.prevent` en la barra no es decoración: sin él la página se desplaza
		// además de activar la fila.
		expect(texto).toContain('@keydown.space.prevent');
	});

	test('el grupo de notificaciones dice si está desplegado', () => {
		// Despliega y repliega: sin esto se anuncia como un botón cualquiera y
		// no se sabe que hay algo plegado detrás.
		expect(leer('components/cards/NotificationGroupCard.vue')).toContain(
			':aria-expanded="isExpanded"'
		);
	});

	test('el selector de audio es un grupo de opciones, no cinco botones iguales', () => {
		// Ésta no tenía botones adentro, así que va `<button>` de verdad. Y
		// elegir una salida es elegir **una de varias**: sin `radio` se leen
		// cinco botones iguales y no se sabe cuál está puesta, que es
		// justamente lo que dibuja el punto de la izquierda.
		const texto = leer('components/controls/AudioDeviceSelector.vue');

		expect(texto).toContain('role="radiogroup"');
		expect(texto).toMatch(/<button[^>]*role="radio"/s);
		expect(texto).toContain(':aria-checked="selectedDeviceId === device.id"');
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
