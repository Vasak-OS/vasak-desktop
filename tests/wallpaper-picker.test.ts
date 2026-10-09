/**
 * El selector rápido de fondos (vasak-desktop#133).
 *
 * Lo que pide el issue y le toca a la aplicación —la navegación del carrusel la
 * prueba la librería, en `tests/wallpaper-carousel.test.ts` de vue-libvasak—:
 *
 * - que al aplicar se escriba **exactamente la clave que escribe
 *   Configuración**: `desktop.wallpaper = [ruta]`, sin tocar el resto;
 * - que un video se aplique ya preparado, con la ruta que devuelve la
 *   preparación de Configuración;
 * - que nunca haya más de una previsualización de video viva.
 *
 * Y lo que lo engancha al escritorio: la ruta, la ventana en la capability, el
 * método de D-Bus, el menú del clic derecho y el CSP que deja reproducir blobs.
 */

import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import type { VSKConfig } from '@vasakgroup/plugin-config-manager';
import {
	allowWallpaperAsset,
	applyWallpaper,
	currentWallpaper,
	customWallpaperFolder,
	loadWallpaperCatalog,
	toCarouselItems,
	wallpaperAssetUrl,
	wallpaperLabel,
	withWallpaper,
} from '../src/services/wallpaper.service';
import {
	createPreviewLoader,
	fetchVideoBlob,
	MAX_VIDEO_BYTES,
	VideoTooLargeError,
} from '../src/utils/video-blob';

const ROOT = join(import.meta.dir, '..');
const read = (file: string) => readFileSync(join(ROOT, file), 'utf8');

const CONFIG: VSKConfig = {
	style: { darkmode: true, 'color-scheme': 'vasak-default', radius: 12 },
	desktop: {
		wallpaper: ['/usr/share/backgrounds/vasakos/wallpaper-1.jpg'],
		iconsize: 48,
		showfiles: true,
		showhiddenfiles: false,
		pausevideoonbattery: false,
		widgets: [{ type: 'clock', x: 0, y: 0 }],
	},
	fonts: { terminal: 'mono', title: 'sans', apps: 'sans' },
	icons: { dark: 'Vasak-dark', light: 'Vasak' },
};

/**
 * Lo que escribe Configuración al guardar un fondo (`WallpaperView.vue` de
 * vasak-settings), copiado de su `saveWallpaperConfig` sin la pausa con batería,
 * que es un interruptor de esa pantalla y acá se deja como está.
 */
function whatSettingsWrites(config: VSKConfig, finalPath: string): VSKConfig {
	return { ...config, desktop: { ...config.desktop, wallpaper: finalPath ? [finalPath] : [] } };
}

describe('aplicar un fondo escribe la clave de Configuración', () => {
	test('desktop.wallpaper pasa a ser [ruta] y el resto queda igual', () => {
		const written = withWallpaper(CONFIG, '/home/pato/fondo.png');

		expect(written).toEqual(whatSettingsWrites(CONFIG, '/home/pato/fondo.png'));
		expect(written.desktop.wallpaper).toEqual(['/home/pato/fondo.png']);
		expect(written.desktop.pausevideoonbattery).toBe(false);
		expect(written.desktop.widgets).toEqual(CONFIG.desktop.widgets);
		expect(written.style).toEqual(CONFIG.style);
		// No muta la que leyó.
		expect(CONFIG.desktop.wallpaper).toEqual(['/usr/share/backgrounds/vasakos/wallpaper-1.jpg']);
	});

	test('el fondo actual sale del primer elemento de la lista', () => {
		expect(currentWallpaper(CONFIG)).toBe('/usr/share/backgrounds/vasakos/wallpaper-1.jpg');
		expect(currentWallpaper({ desktop: { ...CONFIG.desktop, wallpaper: [] } })).toBeNull();
		expect(currentWallpaper({ desktop: { ...CONFIG.desktop, wallpaper: [' '] } })).toBeNull();
		expect(currentWallpaper(null)).toBeNull();
	});

	test('la carpeta propia sale de la misma clave que escribe Configuración', () => {
		expect(
			customWallpaperFolder({ desktop: { ...CONFIG.desktop, wallpaperfolder: '/mnt/fotos' } })
		).toBe('/mnt/fotos');
		// Sin carpeta elegida, o con una clave que no es una cadena: no hay carpeta.
		expect(customWallpaperFolder(CONFIG)).toBeNull();
		expect(customWallpaperFolder({ desktop: { ...CONFIG.desktop, wallpaperfolder: '  ' } })).toBeNull();
		expect(customWallpaperFolder({ desktop: { ...CONFIG.desktop, wallpaperfolder: 7 } })).toBeNull();
		expect(customWallpaperFolder(null)).toBeNull();
	});
});

describe('aplicar, de punta a punta contra el backend', () => {
	const calls: { cmd: string; args: unknown }[] = [];
	const scope = globalThis as Record<string, unknown>;
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
			if (cmd === 'prepare_wallpaper') {
				const path = (args as { path: string }).path;
				return path.endsWith('.mp4')
					? { path: '/home/pato/.cache/vasak/wallpapers/fondo-1.mp4', optimized: true, detail: '1920×1080' }
					: { path, optimized: false, detail: '' };
			}
			if (cmd === 'plugin:config-manager|read_config') return JSON.stringify(CONFIG);
			return null;
		});
	});

	const written = () => {
		const call = calls.find(({ cmd }) => cmd === 'plugin:config-manager|write_config');
		return JSON.parse((call?.args as { payload: string }).payload) as VSKConfig;
	};

	test('una imagen: prepara (no hace nada), relee y escribe la clave', async () => {
		const applied = await applyWallpaper('/usr/share/backgrounds/vasakos/wallpaper-3.jpg');

		expect(calls.map(({ cmd }) => cmd)).toEqual([
			'prepare_wallpaper',
			'plugin:config-manager|read_config',
			'plugin:config-manager|write_config',
		]);
		expect(applied).toBe('/usr/share/backgrounds/vasakos/wallpaper-3.jpg');
		expect(written()).toEqual(whatSettingsWrites(CONFIG, '/usr/share/backgrounds/vasakos/wallpaper-3.jpg'));
	});

	test('un video se guarda con la ruta de la copia preparada, como en Configuración', async () => {
		const applied = await applyWallpaper('/home/pato/Videos/lago.mp4');

		expect(calls[0]).toEqual({ cmd: 'prepare_wallpaper', args: { path: '/home/pato/Videos/lago.mp4' } });
		expect(applied).toBe('/home/pato/.cache/vasak/wallpapers/fondo-1.mp4');
		expect(written().desktop.wallpaper).toEqual(['/home/pato/.cache/vasak/wallpapers/fondo-1.mp4']);
	});
});

describe('el catálogo y el acceso a los fondos', () => {
	const scope = globalThis as Record<string, unknown>;
	const previousWindow = scope.window;

	beforeAll(() => {
		if (!previousWindow) scope.window = globalThis;
	});

	afterAll(() => {
		clearMocks();
		scope.window = previousWindow;
	});

	test('el catálogo le pasa al backend el fondo actual y la carpeta propia', async () => {
		const calls: { cmd: string; args: unknown }[] = [];
		mockIPC((cmd, args) => {
			calls.push({ cmd, args });
			if (cmd === 'wallpaper_catalog') return [];
			return null;
		});

		await loadWallpaperCatalog('/o/1.jpg', '/mnt/fotos');

		expect(calls).toEqual([
			{ cmd: 'wallpaper_catalog', args: { current: '/o/1.jpg', customFolder: '/mnt/fotos' } },
		]);
	});

	test('sin carpeta propia, la pasa en null', async () => {
		const calls: { cmd: string; args: unknown }[] = [];
		mockIPC((cmd, args) => {
			calls.push({ cmd, args });
			return cmd === 'wallpaper_catalog' ? [] : null;
		});

		await loadWallpaperCatalog(null);

		expect(calls[0]).toEqual({
			cmd: 'wallpaper_catalog',
			args: { current: null, customFolder: null },
		});
	});

	test('autorizar un fondo devuelve la ruta canónica que da el backend', async () => {
		mockIPC((cmd, args) =>
			cmd === 'allow_wallpaper_asset'
				? `/home/pato/real/${(args as { path: string }).path.split('/').pop()}`
				: null
		);

		expect(await allowWallpaperAsset('/home/pato/enlace/fondo.jpg')).toBe(
			'/home/pato/real/fondo.jpg'
		);
	});

	test('la URL del fondo autoriza primero y usa la ruta canónica', async () => {
		mockIPC((cmd) => (cmd === 'allow_wallpaper_asset' ? '/home/pato/real/fondo.jpg' : null));

		const url = await wallpaperAssetUrl('/home/pato/enlace/fondo.jpg', (p) => `asset://localhost${p}`);
		expect(url).toBe('asset://localhost/home/pato/real/fondo.jpg');
	});

	test('si no se puede autorizar, cae a la ruta tal cual', async () => {
		mockIPC((cmd) => {
			if (cmd === 'allow_wallpaper_asset') throw new Error('no existe');
			return null;
		});

		const url = await wallpaperAssetUrl('/home/pato/fondo.jpg', (p) => `asset://localhost${p}`);
		expect(url).toBe('asset://localhost/home/pato/fondo.jpg');
	});
});

describe('la fila del carrusel', () => {
	test('cada fondo con su nombre, su miniatura por el protocolo y su marca de video', () => {
		const items = toCarouselItems(
			[
				{ path: '/o/wallpaper-1.jpg', thumbnail: '/c/m1.jpg', video: false },
				{ path: '/home/p/lago.mp4', thumbnail: null, video: true },
			],
			(path) => `asset://localhost${path}`
		);

		expect(items).toEqual([
			{ id: '/o/wallpaper-1.jpg', label: 'wallpaper-1', thumbnail: 'asset://localhost/c/m1.jpg', video: false },
			// Sin miniatura no se cae al original de 5K: el carrusel pone el icono.
			{ id: '/home/p/lago.mp4', label: 'lago', thumbnail: null, video: true },
		]);
	});

	test('el nombre es el archivo sin carpeta ni extensión', () => {
		expect(wallpaperLabel('/usr/share/backgrounds/vasakos/default.jpg')).toBe('default');
		expect(wallpaperLabel('/a/b/.oculto')).toBe('.oculto');
		expect(wallpaperLabel('sin-carpeta.webm')).toBe('sin-carpeta');
	});
});

describe('nunca más de una previsualización de video', () => {
	/** Un cargador de mentira que deja resolver cada carga a mano. */
	function harness() {
		const pending = new Map<string, (url: string) => void>();
		const released: string[] = [];
		const changes: Array<[string | null, string | null]> = [];
		const loader = createPreviewLoader({
			load: (id) =>
				new Promise<string>((resolve) => {
					pending.set(id, resolve);
				}),
			release: (url) => released.push(url),
			onChange: (id, url) => changes.push([id, url]),
		});
		const finish = async (id: string) => {
			pending.get(id)?.(`blob:${id}`);
			await Promise.resolve();
			await Promise.resolve();
		};
		return { loader, finish, released, changes };
	}

	test('pasar a otro video suelta el anterior antes de cargar el nuevo', async () => {
		const { loader, finish, released } = harness();

		const first = loader.show('a.mp4');
		await finish('a.mp4');
		await first;
		expect(loader.aliveCount).toBe(1);

		const second = loader.show('b.mp4');
		// Todavía cargando el segundo, el primero ya se soltó.
		expect(released).toEqual(['blob:a.mp4']);
		expect(loader.aliveCount).toBe(0);
		await finish('b.mp4');
		await second;
		expect(loader.aliveCount).toBe(1);
	});

	test('una carga que llega tarde se suelta sin usarse', async () => {
		const { loader, finish, released, changes } = harness();

		const slow = loader.show('a.mp4');
		const fast = loader.show('b.mp4');
		await finish('b.mp4');
		await fast;
		await finish('a.mp4');
		await slow;

		expect(released).toEqual(['blob:a.mp4']);
		expect(loader.aliveCount).toBe(1);
		expect(changes.at(-1)).toEqual(['b.mp4', 'blob:b.mp4']);
	});

	test('pasar a una imagen o cerrar no deja ninguna', async () => {
		const { loader, finish, released } = harness();

		const first = loader.show('a.mp4');
		await finish('a.mp4');
		await first;
		await loader.show(null);
		expect(loader.aliveCount).toBe(0);
		expect(released).toEqual(['blob:a.mp4']);

		const again = loader.show('b.mp4');
		await finish('b.mp4');
		await again;
		loader.clear();
		expect(loader.aliveCount).toBe(0);
		expect(released).toEqual(['blob:a.mp4', 'blob:b.mp4']);
	});
});

describe('leer un video para reproducirlo', () => {
	const respond = (size: number, declared = size) =>
		(async () =>
			new Response(new Uint8Array(Math.min(size, 16)), {
				headers: { 'content-length': String(declared) },
			})) as unknown as typeof fetch;

	test('uno que anuncia más que el tope no se descarga', async () => {
		await expect(fetchVideoBlob('asset://x.mp4', respond(16, MAX_VIDEO_BYTES + 1))).rejects.toBeInstanceOf(
			VideoTooLargeError
		);
	});

	test('uno chico vuelve como blob', async () => {
		const url = await fetchVideoBlob('asset://x.mp4', respond(16));
		expect(url.startsWith('blob:')).toBe(true);
		URL.revokeObjectURL(url);
	});
});

describe('el selector está enganchado al escritorio', () => {
	test('tiene su ruta', () => {
		expect(read('src/routes/index.ts')).toMatch(
			/path: 'wallpaper-picker',\s*component: \(\) => import\('@\/views\/apps\/WallpaperPickerView\.vue'\)/
		);
	});

	test('la ventana está en la capability, o su página no puede llamar al backend', () => {
		const capability = JSON.parse(read('src-tauri/capabilities/default.json'));
		expect(capability.windows).toContain('wallpaper_picker');
		expect(read('src-tauri/src/windows_apps/wallpaper_picker.rs')).toContain(
			'pub const WALLPAPER_PICKER_LABEL: &str = "wallpaper_picker";'
		);
	});

	test('los comandos están registrados con el nombre que usa la página', () => {
		const lib = read('src-tauri/src/lib.rs');
		for (const command of ['toggle_wallpaper_picker', 'hide_wallpaper_picker', 'wallpaper_catalog', 'prepare_wallpaper']) {
			expect(lib).toContain(`            ${command},`);
		}
	});

	test('se abre por D-Bus, para un atajo de Wayfire', () => {
		expect(read('src-tauri/src/dbus_service.rs')).toMatch(/"OpenWallpaperPicker" => \{[\s\S]*?toggle_wallpaper_picker\(/);
	});

	test('el menú del clic derecho del escritorio lo abre', () => {
		const layer = read('src/components/widgets/WidgetLayer.vue');
		expect(layer).toMatch(/case 'wallpaper':\s*(\/\/[^\n]*\n\s*)*await toggleWallpaperPicker\(\);/);
	});

	test('el CSP deja reproducir blobs: si no, ni el fondo en movimiento ni la previsualización arrancan', () => {
		const csp: string = JSON.parse(read('src-tauri/tauri.conf.json')).app.security.csp;
		const media = csp.split(';').find((part) => part.trim().startsWith('media-src')) ?? '';
		expect(media.split(/\s+/)).toContain('blob:');
	});
});
