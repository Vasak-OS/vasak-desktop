/**
 * La carátula del reproductor, desde `useMusicPlayer` (vasak-desktop#165).
 *
 * El disco del applet se quedaba sin carátula mientras el widget del panel la
 * mostraba bien. El applet y el panel son superficies distintas, y una carátula
 * local se arma como un `blob:` **de cada webview**: un `blob:` de otra ventana
 * no resuelve en la del applet, y el `<img>` dispara `error`. Lo que se fija acá
 * es que, ante ese error, la carátula se **vuelve a resolver** desde `artUrl`
 * —que trae los bytes por IPC y arma un `blob:` propio— en lugar de caer al
 * respaldo para siempre; y que un respaldo tardío (el disparo inicial con
 * `artUrl` vacío) no pisa una carátula más nueva.
 *
 * Se monta el composable de verdad —el que tiene la lógica— con sus imports de
 * `@/…` cambiados por dobles, como hace `menu-app-row.test.ts`: así no importa el
 * `core.service` real, que otra prueba simula con `mock.module` y dejaría sin
 * `musicNowPlaying` según el orden en que corran los archivos. Lo único que
 * entrega los bytes es Tauri, doblado con `__TAURI_INTERNALS__`.
 */
import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { flushPromises, mount } from '@vue/test-utils';
import { defineComponent, h, nextTick } from 'vue';

const ROOT = join(import.meta.dir, '..');

/** Cuántas veces se pidieron los bytes de una carátula. */
let artworkCalls = 0;
/** Bytes de una imagen de mentira: el test no mira píxeles, sólo que haya blob. */
const fakeBytes = () => new Uint8Array([0x89, 0x50, 0x4e, 0x47]).buffer;

let workdir = '';
let useMusicPlayer: () => {
	musicInfo: { value: { artUrl: string } };
	imgSrc: { value: string };
	hasCover: { value: boolean };
	onImgError: () => Promise<void>;
};

beforeAll(async () => {
	(globalThis as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
		metadata: {
			currentWindow: { label: 'applet_music' },
			currentWebview: { label: 'applet_music' },
		},
		transformCallback: () => 0,
		convertFileSrc: (path: string) => path,
		invoke: async (cmd: string) => {
			if (cmd === 'music_artwork') {
				artworkCalls += 1;
				return fakeBytes();
			}
			if (cmd === 'plugin:vicons|get_icon' || cmd === 'plugin:vicons|get_symbol') return '';
			if (cmd === 'plugin:event|listen') return 1;
			return '';
		},
	};
	(globalThis as unknown as { __TAURI_EVENT_PLUGIN_INTERNALS__: unknown }).__TAURI_EVENT_PLUGIN_INTERNALS__ =
		{ unregisterListener: () => {} };

	const cache = join(ROOT, 'node_modules', '.cache');
	mkdirSync(cache, { recursive: true });
	workdir = mkdtempSync(join(cache, 'music-cover-'));

	// Los dobles de lo que `useMusicPlayer` importa de `@/…`. Lo puro —las cuentas
	// de la posición— es el módulo de verdad; lo que habla con Tauri o con otras
	// ventanas, un doble inerte.
	writeFileSync(
		join(workdir, 'composable-doubles.ts'),
		`export { nextLoop, positionFromRatio, positionNow, progressRatio } from ${JSON.stringify(
			join(ROOT, 'src/utils/playback.ts')
		)};
export const musicNowPlaying = async () => ({});
export const useSharedEvent = () => {};
export const logError = () => {};
export type MusicInfo = Record<string, unknown>;
export type PlayerRef = Record<string, unknown>;
`
	);

	// El composable de verdad, con sus `@/…` apuntando a los dobles.
	const { readFileSync } = await import('node:fs');
	const source = readFileSync(join(ROOT, 'src/tools/composables/useMusicPlayer.ts'), 'utf8').replace(
		/from\s+(['"])@\/[^'"]+\1/g,
		"from './composable-doubles'"
	);
	const copy = join(workdir, 'useMusicPlayer.ts');
	writeFileSync(copy, source);
	useMusicPlayer = (await import(copy)).useMusicPlayer;
});

afterAll(() => {
	delete (globalThis as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
	delete (globalThis as unknown as { __TAURI_EVENT_PLUGIN_INTERNALS__?: unknown })
		.__TAURI_EVENT_PLUGIN_INTERNALS__;
	if (workdir) rmSync(workdir, { recursive: true, force: true });
});

beforeEach(() => {
	artworkCalls = 0;
});

/** Un componente mínimo que expone el composable para empujarlo desde el test. */
function mountPlayer() {
	let api!: ReturnType<typeof useMusicPlayer>;
	const Host = defineComponent({
		setup() {
			api = useMusicPlayer();
			return () => h('img', { src: api.imgSrc.value });
		},
	});
	const wrapper = mount(Host);
	return { wrapper, api: () => api };
}

describe('la carátula del reproductor', () => {
	test('una carátula local se arma como blob de este webview', async () => {
		const { wrapper, api } = mountPlayer();
		api().musicInfo.value.artUrl = 'file:///tmp/cover.png';
		await nextTick();
		await flushPromises();

		expect(api().hasCover.value).toBe(true);
		expect(api().imgSrc.value.startsWith('blob:')).toBe(true);
		expect(artworkCalls).toBe(1);
		wrapper.unmount();
	});

	test('si el <img> no carga, la vuelve a resolver desde artUrl (no cae al respaldo)', async () => {
		const { wrapper, api } = mountPlayer();
		api().musicInfo.value.artUrl = 'file:///tmp/cover.png';
		await nextTick();
		await flushPromises();
		expect(artworkCalls).toBe(1);

		// El webview no pudo cargar lo que había (p. ej. un blob de otra ventana).
		await api().onImgError();
		await flushPromises();

		// Se pidieron los bytes de nuevo y quedó un blob propio, no el respaldo.
		expect(artworkCalls).toBe(2);
		expect(api().hasCover.value).toBe(true);
		expect(api().imgSrc.value.startsWith('blob:')).toBe(true);
		wrapper.unmount();
	});

	test('si vuelve a fallar la misma carátula, recién ahí cae al respaldo', async () => {
		const { wrapper, api } = mountPlayer();
		api().musicInfo.value.artUrl = 'file:///tmp/cover.png';
		await nextTick();
		await flushPromises();

		await api().onImgError();
		await flushPromises();
		// Segundo fallo de la MISMA carátula: ya se reintentó, no se reintenta más.
		await api().onImgError();
		await flushPromises();

		expect(api().hasCover.value).toBe(false);
		wrapper.unmount();
	});

	test('una carátula remota no se reintenta: un fallo es la red', async () => {
		const { wrapper, api } = mountPlayer();
		api().musicInfo.value.artUrl = 'https://example.com/cover.png';
		await nextTick();
		await flushPromises();

		expect(api().hasCover.value).toBe(true);
		expect(api().imgSrc.value).toBe('https://example.com/cover.png');
		expect(artworkCalls).toBe(0);

		await api().onImgError();
		await flushPromises();
		// No se pidieron bytes: para lo remoto, al respaldo directo.
		expect(artworkCalls).toBe(0);
		expect(api().hasCover.value).toBe(false);
		wrapper.unmount();
	});
});
