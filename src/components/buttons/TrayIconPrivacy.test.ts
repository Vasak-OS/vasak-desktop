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

const ROOT = join(import.meta.dir, '..', '..', '..');
const COMPONENT = readFileSync(join(import.meta.dir, 'TrayIconPrivacy.vue'), 'utf8');
const APPLET = readFileSync(join(ROOT, 'src', 'views', 'applets', 'PrivacyAppletView.vue'), 'utf8');
const PANEL = readFileSync(
	join(ROOT, 'src', 'components', 'areas', 'panel', 'TrayBarArea.vue'),
	'utf8'
);
const CONFIG = readFileSync(join(ROOT, 'src', 'tools', 'composables', 'usePanelConfig.ts'), 'utf8');
const ROUTES = readFileSync(join(ROOT, 'src', 'routes', 'index.ts'), 'utf8');

describe('quién te mira, te escucha y te ve la pantalla', () => {
	test('sólo aparece cuando hay algo en uso', () => {
		expect(COMPONENT).toContain('const visible = computed(() => symbols.value.length > 0);');
		expect(COMPONENT).toContain('v-if="visible"');
	});

	test('los tres dispositivos tienen su propio símbolo', () => {
		// Un glifo combinado por caso serían siete dibujos —las siete
		// combinaciones de tres— y a 16 píxeles no se distinguen entre sí.
		// Los nombres salen de la lista que arma el componente, que es donde
		// viven desde que el icono se pide por nombre y lo dibuja `ThemeIcon`.
		// Antes se leían de las llamadas a `useSymbol('…')`, que ya no existen:
		// con el patrón viejo esta prueba habría pasado sobre una lista vacía,
		// porque `toEqual([])` contra `[]` es verdad.
		const symbols = [...COMPONENT.matchAll(/icon: '([a-z-]+)'/g)].map(([, s]) => s);

		expect(symbols).toEqual(['camera-web', 'microphone-sensitivity-high', 'video-display']);
		expect(new Set(symbols).size).toBe(3);
	});

	test('se dibuja uno por dispositivo en uso, no uno solo', () => {
		expect(COMPONENT).toContain('v-for="symbol in symbols"');
	});

	test('y cada uno pide el glifo monocromo, no el de color', () => {
		// A dieciséis píxeles en la bandeja, el icono a color es una mancha. Y
		// pedir la variante que no está **no falla**: dibuja otra cosa.
		expect(COMPONENT).toContain('type="symbol"');
	});

	test('nombra a todas las aplicaciones de las tres listas', () => {
		expect(COMPONENT).toContain('[...camera.value, ...microphone.value, ...screen.value].map');
	});

	test('ahora sí se puede hacer clic, y abre el applet', () => {
		// Antes no: no había nada que revocar. Con la captura de pantalla sí lo
		// hay, y el diálogo del portal lo viene prometiendo.
		expect(COMPONENT).toContain('@click="open"');
		expect(COMPONENT).toContain("toggleApplet('privacy', button.value)");
		expect(ROUTES).toContain("path: 'privacy'");
	});

	test('la respuesta de la consulta inicial no pisa un anuncio más nuevo', () => {
		const mounting = COMPONENT.slice(COMPONENT.indexOf('onMounted('));

		expect(mounting).toContain('if (announced.value) return;');
	});

	test('se puede apagar desde la configuración del panel', () => {
		expect(CONFIG).toContain('section.value.privacy !== false');
		expect(PANEL).toContain('<TrayIconPrivacy v-if="showPrivacy"');
	});

	test('los textos están en los dos idiomas', () => {
		for (const language of ['es', 'en']) {
			const yml = readFileSync(join(ROOT, 'src-tauri', 'locales', `${language}.yml`), 'utf8');
			const block = yml.slice(yml.indexOf('  TrayIconPrivacy:'), yml.indexOf('  TrayIconSound:'));

			for (const key of ['camera:', 'microphone:', 'screen:', 'usedBy:']) {
				expect(block).toContain(key);
			}
			expect(yml).toContain('  privacyApplet:');
		}
	});
});

describe('el applet', () => {
	test('sólo la pantalla se puede cortar', () => {
		// La cámara y el micrófono los abre la aplicación contra el dispositivo:
		// no hay nada en el medio que pueda quitárselos, y un botón que no
		// funciona es peor que no tenerlo.
		const buttons = [...APPLET.matchAll(/@click="stopScreen\(/g)];

		expect(buttons).toHaveLength(1);
		expect(APPLET.slice(APPLET.indexOf('screen.length > 0'))).toContain('@click="stopScreen(');
	});

	test('no saca la sesión de la lista por su cuenta', () => {
		// Creerle a la interfaz antes que al agente es exactamente cómo se
		// termina diciendo que dejaste de compartir sin que sea cierto.
		const stopScreen = APPLET.slice(
			APPLET.indexOf('const stopScreen ='),
			APPLET.indexOf('</script>')
		);

		expect(stopScreen).toContain('privacyStopScreen');
		expect(stopScreen).not.toContain('screen.value =');
		expect(stopScreen).not.toContain('.splice(');
		expect(stopScreen).not.toContain('.filter(');
	});

	test('vuelve a preguntar al reabrirse', () => {
		// Esconder no destruye el webview, así que Vue no se monta de nuevo.
		expect(APPLET).toContain('<AppletPopover applet="privacy" @shown="load">');
	});
});
