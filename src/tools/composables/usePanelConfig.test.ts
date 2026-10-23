import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { createPinia, setActivePinia } from 'pinia';
import { DEFAULT_PINNED } from '@/tools/pinned-apps';
import { usePanelConfig } from './usePanelConfig';

/**
 * Los lectores de la sección `panel` (vasak-desktop#251).
 *
 * Todo arranca encendido: la ausencia de una clave significa «mostralo», se lee
 * tolerante (`!== false`). Se prueba contra el store de configuración de verdad,
 * con el `config` puesto a mano: así se comprueba que la regla sea la de
 * producción y no una copia.
 */

/** Pone un `config` cualquiera en el store y devuelve los lectores. */
function withConfig(config: unknown) {
	const store = useConfigStore();
	(store as unknown as { config: unknown }).config = config;
	return usePanelConfig();
}

beforeAll(() => {
	// Algunas dependencias de `@tauri-apps/api` miran esto al importarse.
	(globalThis as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
		metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
		transformCallback: () => 0,
		invoke: async () => null,
	};
});

afterAll(() => {
	delete (globalThis as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
});

beforeEach(() => {
	setActivePinia(createPinia());
});

const BOOLEANS = [
	['showWorkspaces', 'workspaces'],
	['showWindows', 'windows'],
	['showNetworkName', 'networkName'],
	['showVolumeNumber', 'volumeNumber'],
	['showBatteryNumber', 'batteryNumber'],
	['showResources', 'resources'],
] as const;

describe('los interruptores del panel', () => {
	test('sin la sección `panel`, todo encendido', () => {
		const panel = withConfig({});
		for (const [reader] of BOOLEANS) {
			expect(panel[reader].value, reader).toBe(true);
		}
	});

	test('sin esa clave puntual, encendido (ausente = mostralo)', () => {
		const panel = withConfig({ panel: {} });
		for (const [reader] of BOOLEANS) {
			expect(panel[reader].value, reader).toBe(true);
		}
	});

	test('sólo `false` lo apaga; `true` o cualquier otro valor lo deja encendido', () => {
		for (const [reader, key] of BOOLEANS) {
			expect(withConfig({ panel: { [key]: false } })[reader].value, `${key}=false`).toBe(false);
			expect(withConfig({ panel: { [key]: true } })[reader].value, `${key}=true`).toBe(true);
			// Un valor que no sea `false` cuenta como encendido: no es `=== true`.
			expect(withConfig({ panel: { [key]: 0 } })[reader].value, `${key}=0`).toBe(true);
		}
	});

	test('apagar una no apaga las otras', () => {
		const panel = withConfig({ panel: { windows: false } });
		expect(panel.showWindows.value).toBe(false);
		expect(panel.showWorkspaces.value).toBe(true);
		expect(panel.showNetworkName.value).toBe(true);
	});
});

describe('la lista de anclados', () => {
	test('sin `panel.pinned` van los dos por defecto', () => {
		expect(withConfig({ panel: {} }).pinnedApps.value).toBe(DEFAULT_PINNED);
	});

	test('una lista vacía es ninguno, distinto de ausente', () => {
		expect(withConfig({ panel: { pinned: [] } }).pinnedApps.value).toEqual([]);
	});

	test('una lista presente se usa tal cual', () => {
		const pinned = [{ id: '/x.desktop', name: 'X', icon: 'x', exec: 'x' }];
		expect(withConfig({ panel: { pinned } }).pinnedApps.value).toEqual(pinned);
	});
});
