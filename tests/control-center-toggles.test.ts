import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Los botones grandes del centro de control (red, Bluetooth, tema) dibujaban a
 * mano, en `absolute` encima de `ToggleControl`, un punto de estado, un
 * contador y las barras de señal, y le ponían anillos de color por
 * `custom-class`. Desde vue-libvasak 2.2.0 eso es `indicator`, `badge` y la
 * ranura `overlay` del propio botón: el estado entra en su nombre accesible y
 * la forma es la de la librería (vue-libvasak#74).
 */
/** Saca los comentarios HTML cortando por sus delimitadores, sin expresiones regulares. */
function stripHtmlComments(text: string): string {
	let out = '';
	let index = 0;
	while (index < text.length) {
		const start = text.indexOf('<!--', index);
		if (start === -1) return out + text.slice(index);
		out += text.slice(index, start);
		const end = text.indexOf('-->', start + 4);
		if (end === -1) return out;
		index = end + 3;
	}
	return out;
}

const ROOT = join(import.meta.dir, '..');
const read = (file: string) => readFileSync(join(ROOT, 'src/components/controls', file), 'utf8');
const template = (text: string) => text.slice(text.indexOf('<template>'), text.lastIndexOf('</template>'));

const FILES = ['NetworkControl.vue', 'BluetoothControl.vue', 'ThemeToggle.vue'];

describe('los botones del centro de control', () => {
	for (const file of FILES) {
		test(`${file}: el punto es el indicador de la librería`, () => {
			const view = template(read(file));
			expect(view).toContain(':indicator="indicator"');
			// La raíz es el botón: nada dibujado encima a mano ni un envoltorio.
			expect(stripHtmlComments(view.slice('<template>'.length)).trimStart().startsWith('<ToggleControl')).toBe(true);
			expect(view).not.toMatch(/custom-class/);
			expect(view).not.toMatch(/\bring-\d/);
		});
	}

	test('el contador de Bluetooth es el de la librería', () => {
		const view = template(read('BluetoothControl.vue'));
		expect(view).toContain(':badge="connectedDevicesCount"');
		expect(view).not.toContain('{{ connectedDevicesCount }}');
	});

	test('las barras de señal van en la ranura overlay, con alturas de la escala', () => {
		const network = read('NetworkControl.vue');
		expect(template(network)).toContain('#overlay');
		expect(template(network)).not.toMatch(/:style=/);
		expect(network).toContain("['h-1.5', 'h-2', 'h-2.5', 'h-3']");
	});

	test('el estado de la red se dice, no sólo se pinta', () => {
		const network = read('NetworkControl.vue');
		for (const key of ['connected', 'vpn', 'disconnected']) {
			expect(network).toContain(`components.NetworkControl.${key}`);
		}
		for (const locale of ['es', 'en']) {
			const catalog = Bun.YAML.parse(
				readFileSync(join(ROOT, `src-tauri/locales/${locale}.yml`), 'utf8')
			) as { components: { NetworkControl: Record<string, string> } };
			expect(Object.keys(catalog.components.NetworkControl).sort()).toEqual(['connected', 'disconnected', 'vpn']);
		}
	});
});
