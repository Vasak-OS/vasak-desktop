import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { createPinia, setActivePinia } from 'pinia';
import { useMenuConfig } from '@/tools/composables/useMenuConfig';

/**
 * La configuración del menú atada al store (vasak-desktop#203): lee la sección
 * `menu` reactiva y la persiste preservando las claves ajenas. Se prueba con un
 * `__TAURI_INTERNALS__` de mentira que captura lo que se escribe (`write_config`)
 * y un pinia nuevo por prueba.
 */
let writes: Record<string, any>[] = [];
const storedConfig = '{}';

beforeAll(() => {
	(globalThis as any).__TAURI_INTERNALS__ = {
		metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
		transformCallback: () => 0,
		invoke: async (cmd: string, args: any) => {
			if (cmd === 'plugin:config-manager|read_config') return storedConfig;
			if (cmd === 'plugin:config-manager|write_config') {
				writes.push(JSON.parse(args.payload));
				return null;
			}
			if (cmd.startsWith('plugin:i18n')) return cmd.endsWith('get_locale') ? 'es' : {};
			return '';
		},
	};
});

afterAll(() => {
	delete (globalThis as any).__TAURI_INTERNALS__;
});

beforeEach(() => {
	setActivePinia(createPinia());
	writes = [];
});

describe('useMenuConfig', () => {
	test('sin configuración, la sección sale de fábrica', () => {
		const { menu } = useMenuConfig();
		expect(menu.value.variant).toBe('compact');
		expect(menu.value.widget).toBe('weather');
		expect(menu.value.favorites).toEqual([]);
	});

	test('guardar mezcla el cambio en la sección menu y lo persiste', async () => {
		const { setMenuConfig } = useMenuConfig();
		await setMenuConfig({ showUser: false, variant: 'grid' });

		expect(writes).toHaveLength(1);
		expect(writes[0]?.menu.showUser).toBe(false);
		expect(writes[0]?.menu.variant).toBe('grid');
	});

	test('fijar un favorito lo agrega a la lista guardada', async () => {
		const { toggleFavoritePath } = useMenuConfig();
		await toggleFavoritePath('/usr/share/applications/firefox.desktop');

		expect(writes).toHaveLength(1);
		expect(writes[0]?.menu.favorites).toEqual(['/usr/share/applications/firefox.desktop']);
	});
});
