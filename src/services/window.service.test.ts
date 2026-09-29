import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { dismissApplet, toggleApplet } from './window.service';

/**
 * Lo que el panel le manda al backend para abrir un applet.
 *
 * Los nombres de los argumentos son el contrato con `commands/applets.rs`:
 * Tauri los empareja por nombre, y uno mal escrito no falla al compilar, llega
 * como argumento que falta y el botón no abre nada.
 */

const calls: { cmd: string; args: unknown }[] = [];
const scope = globalThis as Record<string, unknown>;
// Otra prueba de la corrida puede dejar `window` declarado y vacío: se mira el
// valor, no si la clave existe.
const previousWindow = scope.window;

beforeAll(() => {
	if (!previousWindow) scope.window = globalThis;
});

afterAll(() => {
	clearMocks();
	scope.window = previousWindow;
});

beforeEach(() => {
	calls.length = 0;
	mockIPC((cmd, args) => {
		calls.push({ cmd, args });
	});
});

describe('abrir un applet', () => {
	test('manda el applet y el rectángulo del botón', async () => {
		const button = {
			getBoundingClientRect: () => ({ x: 1650, y: 2, width: 34, height: 34, top: 2 }),
		};

		await toggleApplet('audio', button);

		expect(calls).toEqual([
			{
				cmd: 'toggle_applet',
				args: { applet: 'audio', anchor: { x: 1650, y: 2, width: 34, height: 34 } },
			},
		]);
	});

	test('sin botón manda `null`, y el backend lo centra en el panel', async () => {
		// `undefined` no viaja en el JSON: el argumento llegaría ausente. Con
		// `null`, `Option<AnchorRect>` recibe `None` a propósito.
		await toggleApplet('network');

		expect(calls).toEqual([{ cmd: 'toggle_applet', args: { applet: 'network', anchor: null } }]);
	});
});

describe('cerrar un applet', () => {
	test('nombra cuál, para no cerrar el que se abrió después', async () => {
		await dismissApplet('tray');

		expect(calls).toEqual([{ cmd: 'dismiss_applet', args: { applet: 'tray' } }]);
	});
});
