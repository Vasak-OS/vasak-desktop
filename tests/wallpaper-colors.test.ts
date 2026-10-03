import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import type {
	FollowWallpaperDeps,
	SchemeData,
	VSKConfig,
	WallpaperPixels,
} from '@vasakgroup/plugin-config-manager';
import { readWallpaperState, withWallpaperState } from '@vasakgroup/plugin-config-manager';
import { createWallpaperFollower } from '@/services/wallpaper-colors.service';

/**
 * «Seguir al fondo» desde el escritorio (vasak-settings#134): el seguidor en
 * fila, y que lo corra **una sola** ventana con el único permiso de guardar
 * esquemas. Lo que decide y calcula (`followWallpaper`) se prueba en el
 * plugin; acá, que el escritorio lo use bien. Los dobles no escriben nada.
 */

const ROOT = join(import.meta.dir, '..');
const read = (file: string) => readFileSync(join(ROOT, file), 'utf8');

const scheme = (): SchemeData => {
	const variant = (bg: string, fg: string) => ({
		ui: {
			color: { primary: '#eba0ac', secondary: '#cba6f7' },
			text: { main: fg, muted: fg, 'on-primary': bg },
			background: bg,
			border: bg,
			surface: bg,
		},
		terminal: {
			foreground: fg,
			background: bg,
			cursor: fg,
			ansi: Object.fromEntries(
				[
					'black',
					'red',
					'green',
					'yellow',
					'blue',
					'magenta',
					'cyan',
					'white',
					'brightBlack',
					'brightRed',
					'brightGreen',
					'brightYellow',
					'brightBlue',
					'brightMagenta',
					'brightCyan',
					'brightWhite',
				].map((name) => [name, '#808080'])
			),
		},
	});
	const base = {
		id: 'custom',
		name: 'Personalizado',
		author: 'pato',
		description: '',
		version: '1',
		colors: { dark: variant('#1e1e2e', '#cdd6f4'), light: variant('#eff1f5', '#4c4f69') },
	} as unknown as SchemeData;
	return withWallpaperState(base, {
		follow: true,
		source: '',
		pinned: { dark: [], light: [] },
	});
};

const solid = (r: number, g: number, b: number): WallpaperPixels => {
	const data = new Uint8Array(96 * 54 * 3);
	for (let i = 0; i < data.length; i += 3) {
		data[i] = r;
		data[i + 1] = g;
		data[i + 2] = b;
	}
	return { width: 96, height: 54, data };
};

function harness(wallpaper: string) {
	const saved: SchemeData[] = [];
	const read: string[] = [];
	let onDisk: SchemeData = scheme();
	const config = {
		style: { darkmode: true, 'color-scheme': 'custom', radius: 8 },
		desktop: { wallpaper: [wallpaper], iconsize: 48, showfiles: true, showhiddenfiles: false },
	} as unknown as VSKConfig;
	const deps: FollowWallpaperDeps = {
		readConfig: async () => config,
		loadScheme: async () => JSON.parse(JSON.stringify(onDisk)),
		readPixels: async (path) => {
			read.push(path);
			// Lento a propósito: el segundo pedido llega mientras el primero lee.
			await new Promise((resolve) => setTimeout(resolve, 20));
			return solid(30, 120, 110);
		},
		save: async (next) => {
			saved.push(next);
			onDisk = next;
		},
	};
	return { deps, saved, read };
}

describe('el seguidor del escritorio', () => {
	test('dos config-changed seguidos leen el fondo una sola vez', async () => {
		const h = harness('/fondos/bosque.jpg');
		const outcomes: string[] = [];
		const follower = createWallpaperFollower(h.deps, (outcome) => outcomes.push(outcome));

		// El segundo es el que dispara su propio guardado.
		const [first, second] = await Promise.all([follower.sync(), follower.sync()]);

		expect(first).toBe('applied');
		expect(second).toBe('unchanged');
		expect(h.read).toEqual(['/fondos/bosque.jpg']);
		expect(h.saved).toHaveLength(1);
		expect(readWallpaperState(h.saved[0] ?? null).source).toBe('/fondos/bosque.jpg');
		expect(outcomes).toEqual(['applied', 'unchanged']);
	});

	test('un error no frena a los siguientes', async () => {
		const h = harness('/fondos/bosque.jpg');
		let fail = true;
		const original = h.deps.readConfig;
		h.deps.readConfig = async () => {
			if (fail) {
				fail = false;
				throw new Error('no se pudo leer vasak.conf');
			}
			return original();
		};
		const follower = createWallpaperFollower(h.deps);
		await expect(follower.sync()).rejects.toThrow();
		expect(await follower.sync()).toBe('applied');
	});
});

describe('una sola ventana sigue al fondo', () => {
	test('DesktopView lo corre sólo fuera de los monitores secundarios', () => {
		const view = read('src/views/DesktopView.vue');
		const follow = view.slice(view.indexOf('const followWallpaper = () => {'));
		expect(follow.slice(0, 200)).toContain('if (isSecondaryMonitor.value) return;');
		// Al montar y en cada config-changed.
		expect(view.match(/followWallpaper\(\);/g)?.length).toBe(2);
	});

	test('nadie más en la página llama a followWallpaper', () => {
		const glob = new Bun.Glob('src/**/*.{ts,vue}');
		const callers = [...glob.scanSync(ROOT)].filter(
			(file) =>
				file !== 'src/services/wallpaper-colors.service.ts' &&
				read(file).includes('createWallpaperFollower(')
		);
		expect(callers.sort()).toEqual(['src/views/DesktopView.vue']);
	});

	test('el permiso de guardar esquemas es sólo de la ventana del monitor principal', () => {
		const capability = JSON.parse(read('src-tauri/capabilities/wallpaper-colors.json'));
		expect(capability.windows).toEqual(['desktop']);
		expect(capability.permissions).toEqual(['config-manager:allow-save-user-scheme']);
		for (const file of ['default.json', 'context-menu.json']) {
			expect(read(`src-tauri/capabilities/${file}`)).not.toContain('allow-save-user-scheme');
		}
	});

	test('el comando de los píxeles está registrado', () => {
		expect(read('src-tauri/src/lib.rs')).toContain('wallpaper_pixels,');
	});
});
