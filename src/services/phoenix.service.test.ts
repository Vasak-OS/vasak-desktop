/**
 * El puente con los comandos de Phoenix (`commands/phoenix.rs`): qué comando se
 * llama y con qué argumentos, que es lo que se separa sin avisar cuando se
 * renombra un lado y no el otro.
 */
import { afterAll, beforeAll, describe, expect, test } from 'bun:test';

const calls: Array<{ cmd: string; args: Record<string, unknown> }> = [];
let service: typeof import('@/services/phoenix.service');

/**
 * El `window` que había antes, para devolverlo al terminar: con el DOM del
 * `preload` (`tests/support/dom.ts`) es el de happy-dom, y borrarlo deja sin
 * `window` a las pruebas que montan componentes después.
 */
const previousWindow = (globalThis as any).window;

beforeAll(async () => {
	(globalThis as any).window = {
		__TAURI_INTERNALS__: {
			transformCallback: () => 1,
			invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
				calls.push({ cmd, args });
				if (cmd === 'phoenix_info') {
					return { installed: false, status: '', connected: false, resources: [] };
				}
				return null;
			},
		},
	};
	service = await import('@/services/phoenix.service');
});

afterAll(() => {
	(globalThis as any).window = previousWindow;
});

describe('el servicio de Phoenix', () => {
	test('el estado se pide con phoenix_info, sin argumentos', async () => {
		const info = await service.getPhoenixInfo();

		expect(info).toEqual({ installed: false, status: '', connected: false, resources: [] });
		expect(calls.at(-1)).toEqual({ cmd: 'phoenix_info', args: {} });
	});

	test('autorizar un recurso es phoenix_authorize con su nombre', async () => {
		await service.authorizePhoenixResource('main-read-replica');

		expect(calls.at(-1)).toEqual({
			cmd: 'phoenix_authorize',
			args: { resource: 'main-read-replica' },
		});
	});
});
