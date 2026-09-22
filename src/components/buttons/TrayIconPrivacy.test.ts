import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * El indicador de cámara, micrófono y pantalla.
 *
 * Lo que se fija acá no son detalles de pintura: son las promesas que el icono
 * le hace a quien lo mira —aparece sólo si pasa algo, distingue los tres
 * dispositivos, y nombra a todos los que los están usando, no al primero— y la
 * que hace el applet, que es la única que se puede incumplir de forma peligrosa:
 * decir que dejaste de compartir sin que sea cierto.
 */

const RAIZ = join(import.meta.dir, '..', '..', '..');
const COMPONENTE = readFileSync(join(import.meta.dir, 'TrayIconPrivacy.vue'), 'utf8');
const APPLET = readFileSync(
	join(RAIZ, 'src', 'views', 'applets', 'PrivacidadAppletView.vue'),
	'utf8'
);
const PANEL = readFileSync(
	join(RAIZ, 'src', 'components', 'areas', 'panel', 'TrayBarArea.vue'),
	'utf8'
);
const CONFIG = readFileSync(join(RAIZ, 'src', 'tools', 'composables', 'usePanelConfig.ts'), 'utf8');
const RUTAS = readFileSync(join(RAIZ, 'src', 'routes', 'index.ts'), 'utf8');

describe('quién te mira, te escucha y te ve la pantalla', () => {
	test('sólo aparece cuando hay algo en uso', () => {
		expect(COMPONENTE).toContain('const visible = computed(() => simbolos.value.length > 0);');
		expect(COMPONENTE).toContain('v-if="visible"');
	});

	test('los tres dispositivos tienen su propio símbolo', () => {
		// Un glifo combinado por caso serían siete dibujos —las siete
		// combinaciones de tres— y a 16 píxeles no se distinguen entre sí.
		// Los nombres salen de la lista que arma el componente, que es donde
		// viven desde que el icono se pide por nombre y lo dibuja `ThemeIcon`.
		// Antes se leían de las llamadas a `useSymbol('…')`, que ya no existen:
		// con el patrón viejo esta prueba habría pasado sobre una lista vacía,
		// porque `toEqual([])` contra `[]` es verdad.
		const simbolos = [...COMPONENTE.matchAll(/icono: '([a-z-]+)'/g)].map(([, s]) => s);

		expect(simbolos).toEqual(['camera-web', 'microphone-sensitivity-high', 'video-display']);
		expect(new Set(simbolos).size).toBe(3);
	});

	test('se dibuja uno por dispositivo en uso, no uno solo', () => {
		expect(COMPONENTE).toContain('v-for="simbolo in simbolos"');
	});

	test('y cada uno pide el glifo monocromo, no el de color', () => {
		// A dieciséis píxeles en la bandeja, el icono a color es una mancha. Y
		// pedir la variante que no está **no falla**: dibuja otra cosa.
		expect(COMPONENTE).toContain('type="symbol"');
	});

	test('nombra a todas las aplicaciones de las tres listas', () => {
		expect(COMPONENTE).toContain('[...camara.value, ...microfono.value, ...pantalla.value].map');
	});

	test('ahora sí se puede hacer clic, y abre el applet', () => {
		// Antes no: no había nada que revocar. Con la captura de pantalla sí lo
		// hay, y el diálogo del portal lo viene prometiendo.
		expect(COMPONENTE).toContain('@click="abrir"');
		expect(COMPONENTE).toContain('togglePrivacyApplet');
		expect(RUTAS).toContain("path: 'privacidad'");
	});

	test('la respuesta de la consulta inicial no pisa un anuncio más nuevo', () => {
		const montaje = COMPONENTE.slice(COMPONENTE.indexOf('onMounted('));

		expect(montaje).toContain('if (yaLlegoUnAnuncio.value) return;');
	});

	test('se puede apagar desde la configuración del panel', () => {
		expect(CONFIG).toContain('seccion.value.privacy !== false');
		expect(PANEL).toContain('<TrayIconPrivacy v-if="showPrivacy"');
	});

	test('los textos están en los dos idiomas', () => {
		for (const idioma of ['es', 'en']) {
			const yml = readFileSync(join(RAIZ, 'src-tauri', 'locales', `${idioma}.yml`), 'utf8');
			const bloque = yml.slice(yml.indexOf('  TrayIconPrivacy:'), yml.indexOf('  TrayIconSound:'));

			for (const clave of ['camera:', 'microphone:', 'screen:', 'usedBy:']) {
				expect(bloque).toContain(clave);
			}
			expect(yml).toContain('  privacidadApplet:');
		}
	});
});

describe('el applet', () => {
	test('sólo la pantalla se puede cortar', () => {
		// La cámara y el micrófono los abre la aplicación contra el dispositivo:
		// no hay nada en el medio que pueda quitárselos, y un botón que no
		// funciona es peor que no tenerlo.
		const botones = [...APPLET.matchAll(/@click="cortar\(/g)];

		expect(botones.length).toBe(1);
		expect(APPLET.slice(APPLET.indexOf('pantalla.length > 0'))).toContain('@click="cortar(');
	});

	test('no saca la sesión de la lista por su cuenta', () => {
		// Creerle a la interfaz antes que al agente es exactamente cómo se
		// termina diciendo que dejaste de compartir sin que sea cierto.
		const cortar = APPLET.slice(APPLET.indexOf('const cortar ='), APPLET.indexOf('const cerrar'));

		expect(cortar).toContain('privacyStopScreen');
		expect(cortar).not.toContain('pantalla.value =');
		expect(cortar).not.toContain('.splice(');
		expect(cortar).not.toContain('.filter(');
	});

	test('vuelve a preguntar al reabrirse', () => {
		// Esconder no destruye el webview, así que Vue no se monta de nuevo.
		expect(APPLET).toContain("useSharedEvent('window-shown', cargar)");
	});
});
