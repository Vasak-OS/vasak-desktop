/**
 * Los widgets se adaptan a su caja, nunca a la ventana (decisión 8; #269).
 *
 * Una celda de 2×1 y una de 8×3 conviven en la misma ventana, así que un corte
 * por ancho de pantalla (`md:`, `@media`, `vw`) no puede decidir lo que cabe en
 * ninguna de las dos. Se lee del fuente, como el resto de las guardas de diseño.
 */
import { describe, expect, test } from 'bun:test';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const DIR = fileURLToPath(new URL('../src/components/widgets/', import.meta.url));
// `WidgetLayer` es la capa del escritorio, no un widget: su panel de «agregar»
// sí se mide contra la pantalla.
const WIDGETS = readdirSync(DIR).filter((f) => f.endsWith('Widget.vue'));

const VIEWPORT = /(^|[\s"'`])(sm|md|lg|xl|2xl):|@media|\d\s?v[wh]\b|innerWidth|innerHeight/;

describe('los widgets', () => {
	test('hay widgets que mirar', () => {
		expect(WIDGETS.length).toBeGreaterThanOrEqual(4);
	});

	for (const file of WIDGETS) {
		test(`${file} no corta por el tamaño de la ventana`, () => {
			const source = readFileSync(join(DIR, file), 'utf8')
				// Los comentarios pueden nombrar `matchMedia` para explicar por qué no se usa.
				.replace(/<!--[\s\S]*?-->/g, '')
				.replace(/\/\*[\s\S]*?\*\//g, '')
				.replace(/^\s*\/\/.*$/gm, '');
			expect(source).not.toMatch(VIEWPORT);
		});
	}
});
