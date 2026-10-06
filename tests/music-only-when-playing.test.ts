/**
 * La música en el centro de control, sólo cuando algo suena, y el widget con
 * los controles debajo del título (vasak-desktop#176).
 *
 * El `MusicWidget` es el mismo en el centro y en los widgets del escritorio
 * (`WidgetLayer`/`WidgetSlot`), así que lo que se fija del orden vale para los
 * dos. Se monta con los servicios doblados (`support/music-doubles.ts`); los
 * componentes de la librería son los publicados.
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { hasActivePlayer } from '../src/tools/music-widget';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';
import { music } from './support/music-doubles';

const DOUBLES = join(import.meta.dir, 'support', 'music-doubles.ts');
const dom = useDom();

let Widget: any;

beforeAll(async () => {
	Widget = await loadComponent(dom.workdir(), 'src/components/widgets/MusicWidget.vue', DOUBLES, 'MusicWidget');
}, 60_000);

beforeEach(() => music.reset());

const settle = async () => {
	for (let i = 0; i < 4; i++) await nextTick();
};

describe('si hay un reproductor que mostrar', () => {
	test('sonando o en pausa, sí', () => {
		expect(hasActivePlayer({ player: 'org.mpris.MediaPlayer2.vlc', status: 'Playing' })).toBe(true);
		expect(hasActivePlayer({ player: 'org.mpris.MediaPlayer2.vlc', status: 'Paused' })).toBe(true);
	});

	test('detenido o sin reproductor, no', () => {
		expect(hasActivePlayer({ player: 'org.mpris.MediaPlayer2.vlc', status: 'Stopped' })).toBe(false);
		expect(hasActivePlayer({ player: '', status: 'Playing' })).toBe(false);
		expect(hasActivePlayer({ player: '', status: '' })).toBe(false);
	});
});

describe('el widget avisa si hay algo sonando', () => {
	test('con un reproductor sonando avisa que sí, y que no al detenerse', async () => {
		const view = mount(Widget, { attachTo: document.body });
		await settle();
		expect(view.emitted('presence')?.at(-1)).toEqual([true]);

		music.info.value = { ...music.info.value, status: 'Stopped' };
		await settle();
		expect(view.emitted('presence')?.at(-1)).toEqual([false]);

		music.info.value = { ...music.info.value, status: 'Paused' };
		await settle();
		expect(view.emitted('presence')?.at(-1)).toEqual([true]);
		view.unmount();
	});

	test('sin reproductor avisa que no desde el primer momento', async () => {
		music.info.value = { ...music.info.value, player: '', status: '' };
		const view = mount(Widget, { attachTo: document.body });
		await settle();
		expect(view.emitted('presence')).toEqual([[false]]);
		view.unmount();
	});
});

describe('los controles van debajo del título', () => {
	test('título → artista → transporte, en la misma columna', async () => {
		const view = mount(Widget, { attachTo: document.body });
		await settle();
		const transport = view.find('[data-transport]');
		expect(transport.exists()).toBe(true);

		const column = transport.element.parentElement as HTMLElement;
		const text = column.textContent ?? '';
		const title = text.indexOf('Atardecer en la terraza del taller');
		const artist = text.indexOf('Los Pingüinos de Wayfire');
		expect(title).toBeGreaterThanOrEqual(0);
		expect(artist).toBeGreaterThan(title);
		// El transporte es el último hijo de la columna del título, no un
		// hermano de la columna en la fila.
		expect(column.lastElementChild).toBe(transport.element);
		expect(transport.findAll('button')).toHaveLength(3);
		view.unmount();
	});

	test('las secciones que dependen del alto siguen saliendo de la medida', () => {
		// La barra, el álbum y los extras se deciden por `sectionsFor` sobre el
		// alto medido de la caja: mover el transporte no les cambia la cuenta.
		const source = readFileSync(join(ROOT, 'src/components/widgets/MusicWidget.vue'), 'utf8');
		expect(source).toContain('sectionsFor(boxHeight.value)');
		expect(source).toContain('v-if="sections.progress && hasProgress"');
		expect(source).toContain('v-if="sections.album && musicInfo.album"');
		expect(source).toContain('v-if="sections.extras && hasExtras"');
	});
});

describe('el centro de control', () => {
	const VIEW = readFileSync(join(ROOT, 'src/views/ControlCenterView.vue'), 'utf8');

	test('muestra la caja de música sólo con un reproductor activo', () => {
		// `v-show`: el widget sigue montado, que es quien escucha a MPRIS.
		expect(VIEW).toMatch(/<div v-show="musicActive"[^>]*data-music-box/);
		expect(VIEW).toContain('@presence="musicActive = $event"');
		expect(VIEW).toContain('const musicActive = ref(false)');
	});
});
