import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * El indicador de cámara y micrófono en uso.
 *
 * Lo que se fija acá no son detalles de pintura: son las tres promesas que el
 * icono le hace a quien lo mira —aparece sólo si pasa algo, distingue los tres
 * casos, y nombra a todos los que están usando el dispositivo, no al primero.
 */

const RAIZ = join(import.meta.dir, '..', '..', '..');
const COMPONENTE = readFileSync(join(import.meta.dir, 'TrayIconPrivacy.vue'), 'utf8');
const PANEL = readFileSync(
	join(RAIZ, 'src', 'components', 'areas', 'panel', 'TrayBarArea.vue'),
	'utf8'
);
const CONFIG = readFileSync(join(RAIZ, 'src', 'tools', 'composables', 'usePanelConfig.ts'), 'utf8');

describe('quién te mira y quién te escucha', () => {
	test('sólo aparece cuando hay algo usando la cámara o el micrófono', () => {
		expect(COMPONENTE).toContain('camara.value.length > 0 || microfono.value.length > 0');
		expect(COMPONENTE).toContain('v-if="visible"');
	});

	test('los tres casos tienen tres símbolos distintos', () => {
		const simbolos = [...COMPONENTE.matchAll(/return '([a-z-]+)';/g)].map(([, s]) => s);

		expect(simbolos).toEqual([
			'vsk-camera-microphone',
			'camera-web',
			'microphone-sensitivity-high',
		]);
		expect(new Set(simbolos).size).toBe(3);
	});

	test('nombra a todas las aplicaciones, no a la primera', () => {
		// Pueden ser varias a la vez: una videollamada y un grabador encima.
		expect(COMPONENTE).toContain('[...camara.value, ...microfono.value].map');
	});

	test('pregunta el estado al montarse, no sólo escucha', () => {
		// El panel se destruye y se vuelve a crear cuando cambian los monitores.
		// El componente nuevo nace vacío y el escritorio no repite un anuncio
		// igual al anterior, así que quedaría invisible con la cámara encendida.
		expect(COMPONENTE).toContain('onMounted(');
		expect(COMPONENTE).toContain('await privacyInUse<');
	});

	test('la respuesta de esa consulta no pisa un anuncio más nuevo', () => {
		const montaje = COMPONENTE.slice(COMPONENTE.indexOf('onMounted('));

		expect(montaje).toContain('if (yaLlegoUnAnuncio.value) return;');
	});

	test('no se puede hacer clic, porque no hay nada que revocar', () => {
		expect(COMPONENTE).toContain(':interactive="false"');
	});

	test('se puede apagar desde la configuración del panel', () => {
		expect(CONFIG).toContain('showPrivacy');
		// Ausente significa «mostralo», como el resto del panel: si esto se
		// leyera `=== true`, el indicador no aparecería en una instalación
		// nueva y nadie sabría que existe.
		expect(CONFIG).toContain('seccion.value.privacy !== false');
	});

	test('el panel lo respeta', () => {
		expect(PANEL).toContain('<TrayIconPrivacy v-if="showPrivacy"');
	});

	test('los textos están en los dos idiomas', () => {
		const claves = ['camera:', 'microphone:', 'both:', 'usedBy:'];

		for (const idioma of ['es', 'en']) {
			const yml = readFileSync(join(RAIZ, 'src-tauri', 'locales', `${idioma}.yml`), 'utf8');
			const bloque = yml.slice(yml.indexOf('  TrayIconPrivacy:'));

			expect(bloque).toContain('  TrayIconPrivacy:');
			for (const clave of claves) {
				expect(bloque.slice(0, bloque.indexOf('  TrayIconSound:'))).toContain(clave);
			}
		}
	});
});
