/**
 * El ecualizador de sistema al pie del reproductor desplegable
 * (vasak-desktop#131, vasak-wireplumber-modules#10/#12): montado, con
 * `Equalizer` de la librería publicada y `useEqualizer` en un doble.
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { loadComponent, useDom } from './support/mount-sfc';
import { music } from './support/music-doubles';

const DOUBLES = join(import.meta.dir, 'support', 'music-doubles.ts');
const dom = useDom();
// biome-ignore lint/suspicious/noExplicitAny: el componente compilado no trae tipos.
let Applet: any;

beforeAll(async () => {
	Applet = await loadComponent(dom.workdir(), 'src/views/applets/MusicAppletView.vue', DOUBLES, 'MusicAppletViewEq');
}, 60_000);
beforeEach(() => music.reset());

const settle = async () => {
	for (let i = 0; i < 4; i++) await nextTick();
};

const ROCK = {
	service: true,
	frequencies: [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000],
	range: [-12, 12] as [number, number],
	presets: ['flat', 'bass', 'treble', 'vocal', 'pop', 'rock', 'jazz', 'classic'],
	preset: 'rock',
	gains: [4.5, 3.5, 1.5, -1, -2, -1, 1.5, 3, 4, 4.5],
	enabled: true,
	available: true,
	saved: true,
};

async function mountApplet() {
	const view = mount(Applet, { attachTo: document.body });
	await settle();
	return view;
}

describe('el ecualizador al pie de la tarjeta', () => {
	test('va en el pie de NowPlayingCard, con las diez bandas y los perfiles traducidos', async () => {
		music.equalizer.value = ROCK;
		const view = await mountApplet();
		expect(view.find('[data-footer] [data-equalizer]').exists()).toBe(true);
		expect(view.findAll('[data-equalizer] [data-thumb]')).toHaveLength(10);
		const rock = view.find('[data-preset="rock"]');
		expect(rock.attributes('aria-checked')).toBe('true');
		// Sin catálogo en la prueba `t()` devuelve la clave cruda, y la vista cae
		// en el identificador del perfil antes que leer la clave en voz alta.
		expect(rock.text()).toBe('rock');
		view.unmount();
	});

	test('pide el estado cada vez que el applet vuelve a la vista', async () => {
		const view = await mountApplet();
		expect(music.calls.some((entry) => entry.name === 'loadEqualizer')).toBe(true);
		view.unmount();
	});

	test('sin el servicio se ve no disponible, nunca roto', async () => {
		const view = await mountApplet();
		expect(view.find('[data-unavailable]').exists()).toBe(true);
		expect(view.find('[data-equalizer] [data-thumb]').exists()).toBe(false);
		view.unmount();
	});

	test('mover una banda y elegir un perfil van a useEqualizer', async () => {
		music.equalizer.value = ROCK;
		const view = await mountApplet();
		view.find('[data-thumb="2"]').element.dispatchEvent(
			new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true, cancelable: true })
		);
		await view.find('[data-preset="jazz"]').trigger('click');
		expect(music.calls.find((entry) => entry.name === 'setGain')?.args).toEqual([2, 2]);
		expect(music.calls.find((entry) => entry.name === 'setPreset')?.args).toEqual(['jazz']);
		view.unmount();
	});
});
