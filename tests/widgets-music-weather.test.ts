/**
 * Los widgets de música y clima del escritorio, con la estética del panel y del
 * tablero de fecha (vasak-desktop#166, #167).
 *
 * Como `music-applet.test.ts`, se mira el texto y no se monta: lo que se fija es
 * que los widgets usen los componentes de `vue-libvasak` —no un dibujo propio
 * (decisión 8)— y que no abran un segundo camino a los datos. Lo que dibuja cada
 * componente de la librería ya está probado, montado, en `vue-libvasak`.
 */
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = join(import.meta.dir, '..');
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

const MUSIC = read('src/components/widgets/MusicWidget.vue');
const WEATHER = read('src/components/widgets/WeatherWidget.vue');
const ES = Bun.YAML.parse(read('src-tauri/locales/es.yml')) as Record<string, any>;
const EN = Bun.YAML.parse(read('src-tauri/locales/en.yml')) as Record<string, any>;

describe('el widget de música, como el reproductor del panel', () => {
	test('arma la estética con las piezas de la librería, no dibujadas a mano', () => {
		// Las mismas piezas que la `NowPlayingCard` del applet: el disco que gira,
		// la barra y los controles.
		expect(MUSIC).toMatch(
			/import \{[\s\S]*?SpinningCover[\s\S]*?\} from '@vasakgroup\/vue-libvasak'/
		);
		expect(MUSIC).toContain('<SpinningCover');
		expect(MUSIC).toContain('<SeekBar');
		expect(MUSIC).toContain('<ActionButton');
		expect(MUSIC).toContain('<ToggleControl');
	});

	test('la música sale de useMusicPlayer: no hay un segundo camino a MPRIS', () => {
		expect(MUSIC).toContain('useMusicPlayer()');
		expect(MUSIC).not.toMatch(/invoke\s*[<(]/);
		expect(MUSIC).not.toContain("'music_");
	});

	test('el disco distingue pausa de detenido y muestra el avance', () => {
		expect(MUSIC).toContain('playbackStateOf(musicInfo.value.status)');
		expect(MUSIC).toContain(':state="state"');
		expect(MUSIC).toContain(':progress="ringProgress"');
	});

	test('no quedó el dibujo propio de los botones', () => {
		// Los botones de transporte eran `<button>` con clases a mano; ahora son de
		// la librería. No tiene que quedar ninguno en la plantilla.
		expect(MUSIC).not.toContain('<button');
	});
});

describe('el widget de clima, como el clima del tablero de fecha', () => {
	test('usa los anillos de la librería y las cuentas del tablero', () => {
		expect(WEATHER).toMatch(
			/import \{[\s\S]*?ProgressRing[\s\S]*?\} from '@vasakgroup\/vue-libvasak'/
		);
		expect(WEATHER).toContain('<ProgressRing');
		// Las mismas cuentas que el tablero de fecha, no una copia: los anillos
		// salen del ayudante compartido `weatherRings`.
		expect(WEATHER).toMatch(/from '@\/tools\/date-board'/);
		expect(WEATHER).toContain('dayWeather(');
		expect(WEATHER).toContain('weatherRings(');
		expect(WEATHER).toContain('weatherIcon(');
	});

	test('tiene las flechas para ver otros días y las tarjetas pasantes', () => {
		expect(WEATHER).toContain('stepDay(-1)');
		expect(WEATHER).toContain('stepDay(1)');
		// Las tarjetas pasantes son el control segmentado de la librería, no un
		// botón dibujado a mano (decisión 8).
		expect(WEATHER).toContain('<SegmentedControl');
		expect(WEATHER).not.toContain('<button');
	});

	test('el clima sale de useWeather: no hay un segundo pedido', () => {
		expect(WEATHER).toContain('useWeather()');
		expect(WEATHER).not.toMatch(/fetch\(/);
	});

	test('las dos variantes se dibujan igual; today arranca en hoy', () => {
		expect(WEATHER).toContain("props.variant === 'today'");
		// La variante de hoy no deja moverse de día.
		expect(WEATHER).toContain('if (soloHoy.value) return');
	});
});

describe('los textos nuevos están en los dos idiomas', () => {
	test('música: vía, de {0}, reproductores', () => {
		for (const catalog of [ES, EN]) {
			const mw = catalog.components.MusicWidget;
			expect(mw.viaCaption).toBeTruthy();
			expect(mw.byArtist).toContain('{0}');
			expect(mw.players).toBeTruthy();
		}
	});

	test('clima: los cuatro anillos y las flechas', () => {
		for (const catalog of [ES, EN]) {
			const ww = catalog.components.WeatherWidget;
			for (const key of ['wind', 'humidity', 'rain', 'feelsLike', 'previousDay', 'nextDay']) {
				expect(ww[key]).toBeTruthy();
			}
		}
	});
});
