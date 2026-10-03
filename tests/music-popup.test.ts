/**
 * El reproductor desplegable, montado: la píldora del panel que lo abre y la
 * tarjeta que se despliega (vasak-desktop#131).
 *
 * Hasta 1.23 la píldora tenía los comandos adentro —aparecían al pasar el
 * puntero— y lo único que abría el reproductor era la portada de 22 píxeles.
 * Acá se fija lo que quedó: la píldora entera es un botón que pide
 * `toggle_applet('music')` colgado de ella, no tiene comandos, y la tarjeta
 * tiene todo lo del diseño del issue con los estados raros de MPRIS.
 *
 * Los componentes de la librería son los publicados; los servicios del
 * escritorio, dobles (`support/music-doubles.ts`).
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { loadComponent, useDom } from './support/mount-sfc';
import { music } from './support/music-doubles';

const DOUBLES = join(import.meta.dir, 'support', 'music-doubles.ts');
const dom = useDom();

let Pill: any;
let Applet: any;

beforeAll(async () => {
	Pill = await loadComponent(dom.workdir(), 'src/components/controls/TrayMusicControl.vue', DOUBLES, 'TrayMusicControl');
	Applet = await loadComponent(dom.workdir(), 'src/views/applets/MusicAppletView.vue', DOUBLES, 'MusicAppletView');
	// Compilar dos componentes y cargar la librería entera tarda: con la
	// máquina cargada pasa de los cinco segundos de un gancho.
}, 60_000);

beforeEach(() => music.reset());

const settle = async () => {
	for (let i = 0; i < 4; i++) await nextTick();
};

async function mountPill() {
	const view = mount(Pill, { attachTo: document.body });
	await settle();
	return view;
}

async function mountApplet() {
	const view = mount(Applet, { attachTo: document.body });
	await settle();
	return view;
}

describe('la píldora del panel', () => {
	test('es un solo botón: la portada y el título corto', async () => {
		const view = await mountPill();
		const pill = view.find('button');

		expect(pill.element.tagName).toBe('BUTTON');
		expect(view.findAll('button')).toHaveLength(1);
		expect(view.find('[data-music-title]').text()).toBe('Atardecer en la terraza del taller');
		expect(view.find('[data-music-title]').classes()).toContain('truncate');
		view.unmount();
	});

	test('tocarla pide toggle_applet con el id music y la píldora como ancla', async () => {
		const view = await mountPill();
		await view.find('button').trigger('click');
		await settle();

		const call = music.calls.find((entry) => entry.name === 'toggleApplet');
		expect(call?.args[0]).toBe('music');
		// El ancla es el componente del botón; `anchorOf` mide su `$el`.
		const anchor = call?.args[1] as { $el?: Element } | undefined;
		expect(anchor?.$el).toBe(view.find('button').element);
		view.unmount();
	});

	test('no tiene comandos: ni anterior, ni reproducir, ni siguiente', async () => {
		const view = await mountPill();
		await view.trigger('mouseenter');
		await settle();

		expect(view.findAll('button')).toHaveLength(1);
		expect(view.html()).not.toMatch(/media-(skip|seek)-(backward|forward)|media-playback-(start|pause)/);
		expect(music.calls.filter((entry) => /^on(Prev|Next|PlayPause)$/.test(entry.name))).toEqual([]);
		view.unmount();
	});

	test('con el panel a un costado queda la portada sola', async () => {
		music.vertical.value = true;
		const view = await mountPill();
		expect(view.find('[data-music-title]').exists()).toBe(false);
		view.unmount();
	});

	test('sin nada sonando no hay título, y el botón igual tiene nombre', async () => {
		music.info.value = { ...music.info.value, title: '', artist: '', status: 'Stopped' };
		const view = await mountPill();
		expect(view.find('[data-music-title]').exists()).toBe(false);
		expect(view.find('button').attributes('aria-label')).toBeTruthy();
		view.unmount();
	});
});

describe('la tarjeta del reproductor', () => {
	test('título, artista, las dos pastillas, la barra y el transporte', async () => {
		music.devices.value = [
			{ id: 'b', name: 'Auriculares Bluetooth WH-1000XM4', description: 'Auriculares', is_default: true, volume: 0.3 },
		];
		const view = await mountApplet();

		expect(view.find('[data-title]').text()).toBe('Atardecer en la terraza del taller');
		expect(view.find('[data-artist]').exists()).toBe(true);
		const output = view.find('button[data-chip]');
		expect(output.element.tagName).toBe('BUTTON');
		expect(output.text()).toContain('Auriculares Bluetooth WH-1000XM4');
		const via = view.find('span[data-chip]');
		expect(via.element.tagName).toBe('SPAN');
		expect(via.text()).toContain('Reproductor multimedia VLC');
		expect(view.find('[data-seek-bar]').exists()).toBe(true);
		expect(view.find('[data-toggle]').classes()).toContain('bg-primary');
		view.unmount();
	});

	test('la pastilla de la salida abre el selector en lugar de la tarjeta, y volver la trae', async () => {
		music.devices.value = [
			{ id: 'a', name: 'Altavoces', description: 'Altavoces', is_default: true, volume: 0.6 },
			{ id: 'b', name: 'HDMI', description: 'HDMI', is_default: false, volume: 1 },
		];
		const view = await mountApplet();
		await view.find('button[data-chip]').trigger('click');
		await settle();

		expect(view.find('[data-output-picker]').exists()).toBe(true);
		expect(view.find('[data-now-playing]').exists()).toBe(false);
		expect(view.find('[role="radiogroup"]').exists()).toBe(true);

		await view.find('[data-output-picker] button').trigger('click');
		await settle();
		expect(view.find('[data-now-playing]').exists()).toBe(true);
		view.unmount();
	});

	test('sin carátula el disco dibuja el icono de la aplicación', async () => {
		music.cover.value = '';
		const view = await mountApplet();
		expect(view.find('[data-now-playing] img').exists()).toBe(false);
		expect(view.find('[data-spinning-cover] [data-fallback]').exists()).toBe(true);
		// El icono del disco es el del `.desktop` del reproductor.
		const card = view.findComponent({ name: 'NowPlayingCard' });
		expect(card.props('coverSrc')).toBeNull();
		expect(card.props('fallbackIcon')).toBe('vlc');
		view.unmount();
	});

	test('con canSeek en falso la barra se ve pero no se arrastra', async () => {
		music.info.value = { ...music.info.value, canSeek: false };
		const view = await mountApplet();
		expect(view.find('[data-seek-bar] input').attributes('disabled')).toBeDefined();
		view.unmount();
	});

	test('con length en 0 (una radio en vivo) no hay barra, y la tarjeta sigue', async () => {
		music.info.value = { ...music.info.value, length: 0, position: 0, canSeek: false };
		const view = await mountApplet();
		expect(view.find('[data-seek-bar]').exists()).toBe(false);
		expect(view.find('[data-toggle]').exists()).toBe(true);
		view.unmount();
	});

	test('lo que la fuente no permite se ve apagado, no desaparece', async () => {
		music.info.value = { ...music.info.value, canGoNext: false, canGoPrevious: false };
		const view = await mountApplet();
		expect(view.find('[data-previous]').attributes('disabled')).toBeDefined();
		expect(view.find('[data-next]').attributes('disabled')).toBeDefined();
		await view.find('[data-next]').trigger('click');
		expect(music.calls.some((entry) => entry.name === 'onNext')).toBe(false);
		view.unmount();
	});

	test('con un reproductor no hay puntos ni hueco', async () => {
		music.players.value = [
			{ player: 'org.mpris.MediaPlayer2.vlc', identity: 'VLC', status: 'Playing', title: '', active: true, pinned: false },
		];
		const view = await mountApplet();
		expect(view.find('[data-page-dots]').exists()).toBe(false);
		view.unmount();
	});

	test('con varios, un punto por reproductor, el activo en primary, y tocar otro lo elige', async () => {
		music.players.value = [
			{ player: 'org.mpris.MediaPlayer2.firefox.instance_1', identity: 'Firefox', status: 'Paused', title: 'Un video', active: false, pinned: false },
			{ player: 'org.mpris.MediaPlayer2.vlc', identity: 'VLC', status: 'Playing', title: 'Atardecer', active: true, pinned: false },
			{ player: 'org.mpris.MediaPlayer2.spotify', identity: 'Spotify', status: 'Paused', title: '', active: false, pinned: false },
		];
		const view = await mountApplet();
		const dots = view.findAll('[data-dot]');

		expect(dots).toHaveLength(3);
		expect(dots[1]?.attributes('aria-current')).toBe('true');
		expect(dots[1]?.find('span').classes()).toContain('bg-primary');
		expect(dots[0]?.attributes('aria-label')).toBe('Firefox: Un video');

		await dots[2]?.trigger('click');
		await settle();
		expect(music.calls.find((entry) => entry.name === 'selectPlayer')?.args).toEqual(['org.mpris.MediaPlayer2.spotify']);
		view.unmount();
	});

	test('la pausa congela el disco donde está; no le saca la animación', async () => {
		const view = await mountApplet();
		const spinning = () => view.find('[data-now-playing] [style*="animation"]');
		expect(spinning().attributes('style')).toContain('running');

		music.info.value = { ...music.info.value, status: 'Paused' };
		await settle();
		const style = spinning().attributes('style') ?? '';
		// Sigue puesta la misma animación, en pausa: no vuelve a cero.
		expect(style).toContain('paused');
		expect(style).toContain('animation-duration: 8s');
		view.unmount();
	});

	test('sin reproducción el disco no gira', async () => {
		music.info.value = { ...music.info.value, status: 'Stopped' };
		const view = await mountApplet();
		expect(view.find('[data-now-playing] [style*="animation"]').exists()).toBe(false);
		view.unmount();
	});

});
