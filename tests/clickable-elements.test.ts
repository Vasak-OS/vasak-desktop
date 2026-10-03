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

const root = new URL('../src/', import.meta.url).pathname;

const sources = await Promise.all(
	[...new Glob('**/*.vue').scanSync(root)].map(async (path) => ({
		path,
		text: await Bun.file(root + path).text(),
	}))
);

const read = (path: string) => sources.find((f) => f.path === path)?.text ?? '';

/** Etiquetas nativas que no hacen nada por sí solas al recibir un clic. */
// Las comillas no entran en la primera alternativa: si entraran, una cadena de
// comillas se podría repartir de muchas maneras y la búsqueda retrocedería sin
// fin (CodeQL, js/redos).
const DEAF_TAGS = /<(div|span|img|li|p|a)\b((?:[^<>"']|"[^"]*"|'[^']*')*?)>/g;

function deafTagsListeningToClick(text: string): number {
	let count = 0;
	for (const [, , attributes] of text.matchAll(DEAF_TAGS)) {
		if (!/(@click|v-on:click)\b/.test(attributes)) continue;
		// El papel puesto a mano y el atado a una condición valen los dos. Varias
		// de estas filas **sólo a veces** hacen algo —`clickable`, o que la
		// notificación traiga acción por omisión—, y ahí el papel tiene que
		// aparecer y desaparecer con eso: anunciar un botón que no hace nada es
		// el mismo problema al revés. Cuando no lo es, el manejador corta
		// primero, así que el `@click` que queda no hace nada.
		if (/(?:^|\s):?role="[^"]*button/.test(attributes)) continue;
		count += 1;
	}
	return count;
}

describe('nada que se pueda clickear queda fuera del teclado', () => {
	test('ningún elemento sordo escucha el clic — ya sin excepciones', () => {
		// Hubo cinco: las filas enteras de las tarjetas y el selector de audio.
		// Eran otra discusión porque varias tienen **sus propios botones
		// adentro** y un botón dentro de otro no es HTML válido. Se resolvieron
		// caso por caso y la lista quedó vacía, así que acá ya no hay salvedad
		// que hacer.
		const offenders = sources
			.filter(({ text }) => deafTagsListeningToClick(text) > 0)
			.map(({ path }) => path);

		expect(offenders).toEqual([]);
	});
});

/**
 * Que no sea un `<div>` sordo no alcanza: `role="button"` sin `tabindex` no
 * recibe foco, y con foco pero sin manejadores de tecla no se activa. Las tres
 * cosas van juntas o no sirve ninguna — y como nada de esto falla ni avisa,
 * sólo se nota probándolo con el teclado, que es lo que nadie hace.
 */
describe('las filas que se abren enteras', () => {
	// `ListCard` y `DeviceCard` ya no están acá: se fueron a la librería, con su
	// teclado puesto. Ver Vasak-OS/vue-libvasak#22, que es el barrido de las
	// copias.
	const WITH_INNER_BUTTONS = ['components/cards/NotificationCard.vue'];

	test.each(WITH_INNER_BUTTONS)('%s se enfoca y se activa con el teclado', (path) => {
		const text = read(path);

		expect(text).toMatch(/(?::role="|\srole=")/);
		expect(text).toMatch(/(?::tabindex="|\stabindex=")/);
		// `.self` antes de `.prevent`, y las dos cosas importan. Sin `.self`, la
		// tecla apretada sobre un botón de adentro **burbujea** hasta acá: se
		// dispara además la acción de la fila entera y el `.prevent` le cancela
		// al botón su propia activación. Sin `.prevent`, la barra desplaza la
		// página además de activar. Lo del burbujeo lo encontró CodeRabbit en
		// vue-libvasak#62, sobre el mismo patrón.
		expect(text).toContain('@keydown.enter.self.prevent');
		expect(text).toContain('@keydown.space.self.prevent');
	});

	test('el grupo de notificaciones dice si está desplegado', () => {
		// Despliega y repliega: sin `aria-expanded` se anuncia como un botón
		// cualquiera y no se sabe que hay algo plegado detrás. Desde la 2.2.0
		// la cabecera es `Disclosure` de la librería, un `<button>` con
		// `aria-expanded` y `aria-controls`; el de descartar va afuera de él.
		const text = read('components/cards/NotificationGroupCard.vue');
		expect(text).toContain('<Disclosure v-model:open="isExpanded"');
		expect(text).not.toContain('role="button"');
	});

	test('el selector de audio es un grupo de opciones, no cinco botones iguales', () => {
		// Ésta no tenía botones adentro, así que va `<button>` de verdad. Y
		// elegir una salida es elegir **una de varias**: sin `radio` se leen
		// cinco botones iguales y no se sabe cuál está puesta, que es
		// justamente lo que dibuja el punto de la izquierda.
		// Desde vue-libvasak 2.2.0 es `OptionGroup` de la librería, que es
		// `role="radiogroup"` con un `<button role="radio">` por opción, las
		// flechas y un solo Tab (lo prueba la librería montado). Acá queda que
		// las dos copias del escritorio lo pidan a ella.
		for (const path of ['components/controls/AudioDeviceSelector.vue', 'views/applets/MusicAppletView.vue']) {
			const text = read(path);
			expect(text).toMatch(/<OptionGroup\b/);
			expect(text).not.toMatch(/role="radio(group)?"|type="radio"/);
		}
	});
});

describe('los botones del panel se anuncian con nombre', () => {
	/**
	 * El icono de la bandeja se fue a la librería con todo esto puesto.
	 *
	 * Lo que estas pruebas miraban —que el nombre accesible salga del `alt`
	 * o del tooltip, y que el que sólo informa se dibuje como `div`— ahora se
	 * comprueba **montado** en `vue-libvasak`, que es donde vive la conducta.
	 * Acá queda lo que a esta aplicación le toca: que no vuelva a haber copia y
	 * que lo pidan a la librería.
	 *
	 * Con el panel en píldoras (vasak-desktop#151) los botones del propio
	 * panel —la búsqueda, las notificaciones, el teléfono, la red, el
	 * Bluetooth, el volumen, la música, el reloj— pasaron a ser `PanelPill`, y
	 * los espacios de trabajo `WorkspaceSwitcher`. `TrayIconButton` queda para
	 * los cinco iconos chicos que viven adentro de la píldora de la bandeja y
	 * de la de las ventanas: privacidad, Bloq Mayús, micrófono, Twingate y el
	 * botón de cada ventana.
	 */
	test('el de la bandeja ya no tiene copia acá', () => {
		expect(sources.filter(({ path }) => path.endsWith('buttons/TrayIconButton.vue'))).toEqual([]);
	});

	test.each(['TrayIconButton', 'PanelPill', 'WorkspaceSwitcher'])('%s se pide a la librería', (name) => {
		// Si alguno lo usa sin importarlo, Vue dibuja un elemento desconocido y
		// no falla: el icono no está y el panel queda con un hueco.
		const offenders = sources
			.filter(({ text }) => new RegExp(`<${name}\\b`).test(text))
			.filter(
				({ text }) =>
					!new RegExp(`import \\{[^}]*\\b${name}\\b[^}]*\\} from '@vasakgroup\\/vue-libvasak'`).test(text)
			)
			.map(({ path }) => path);

		expect(offenders).toEqual([]);
	});

	test('son cinco los iconos de la bandeja y doce las píldoras, no menos', () => {
		// Sin esto, la de arriba pasa sobre una lista vacía el día que alguien
		// renombre los archivos y el patrón deje de encontrarlos.
		const users = (name: string) =>
			sources.filter(({ text }) => new RegExp(`<${name}\\b`).test(text)).map(({ path }) => path).sort();

		expect(users('TrayIconButton')).toHaveLength(5);
		expect(users('PanelPill')).toEqual([
			'components/areas/panel/TrayBarArea.vue',
			'components/areas/panel/WindowsArea.vue',
			'components/buttons/TrayIconBattery.vue',
			'components/buttons/TrayIconBluetooth.vue',
			'components/buttons/TrayIconNetwork.vue',
			'components/buttons/TrayIconSound.vue',
			'components/controls/TrayMusicControl.vue',
			'components/controls/TrayWeatherControl.vue',
			'components/panel/KeyboardLayoutPill.vue',
			'components/widgets/PanelClockWidget.vue',
			'views/PanelView.vue',
		]);
		expect(users('WorkspaceSwitcher')).toEqual(['components/panel/WorkspacesPill.vue']);
	});

	test('los del panel llevan su nombre puesto', () => {
		// Los iconos chicos le pasan el nombre al de la librería por `alt`; las
		// píldoras que son botón, por `accessible-label`.
		expect(read('components/buttons/TrayIconPrivacy.vue')).toContain(':alt="detail"');
		expect(read('components/buttons/WindowPanelButton.vue')).toContain(':alt="title"');
		const panel = read('views/PanelView.vue');
		expect(panel).toContain(':accessible-label="t(\'views.panel.searchAlt\')"');
		expect(panel).toContain(':accessible-label="t(\'views.panel.notificationsAlt\')"');
		for (const path of [
			'components/buttons/TrayIconNetwork.vue',
			'components/buttons/TrayIconBluetooth.vue',
			'components/buttons/TrayIconSound.vue',
			'components/controls/TrayMusicControl.vue',
			'components/widgets/PanelClockWidget.vue',
			'components/controls/TrayWeatherControl.vue',
			'components/panel/KeyboardLayoutPill.vue',
		]) {
			expect(read(path), path).toMatch(/:accessible-label="/);
		}
	});
});
