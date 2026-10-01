/**
 * El reproductor que se despliega desde el control de música del panel.
 *
 * Lo que dibujan la tarjeta, el disco y la barra está probado en vue-libvasak,
 * montado. Lo que se fija acá es cómo los usa el escritorio, que es lo que se
 * separa sin avisar: un segundo camino a MPRIS al lado de `useMusicPlayer`, una
 * segunda barra de progreso al lado de `SeekBar`, un espacio del ecualizador
 * dibujado vacío, o un applet que abre sin permisos porque su ventana no está
 * en la capability.
 *
 * Se mira el texto y no se monta porque este repositorio todavía no tiene con
 * qué montar, igual que `controles-de-la-libreria.test.ts`.
 */
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = join(import.meta.dir, '..');
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

const VIEW = read('src/views/applets/MusicAppletView.vue');
const CONTROL = read('src/components/controls/TrayMusicControl.vue');
const WIDGET = read('src/components/widgets/MusicWidget.vue');
const CSS = read('src/assets/main.css');
const CAPABILITY = JSON.parse(read('src-tauri/capabilities/default.json')) as {
	windows: string[];
};
const ES = Bun.YAML.parse(read('src-tauri/locales/es.yml')) as Record<string, any>;
const EN = Bun.YAML.parse(read('src-tauri/locales/en.yml')) as Record<string, any>;

describe('el reproductor desplegable', () => {
	test('lo abre el control del panel, colgado de él', () => {
		expect(CONTROL).toContain("toggleApplet('music', opener.value)");
		expect(CONTROL).toContain('ref="opener"');
		// Y el control se realza mientras está abierto, como los demás.
		expect(CONTROL).toContain("useOpenApplet('music')");
		expect(CONTROL).toContain(':class="openClasses"');
	});

	test('es un applet anclado, con su ventana en la capability', () => {
		expect(VIEW).toContain('<AppletPopover applet="music"');
		expect(CAPABILITY.windows).toContain('applet_music');
	});

	test('la música sale de useMusicPlayer: no hay un segundo camino a MPRIS', () => {
		expect(VIEW).toContain('useMusicPlayer()');
		expect(VIEW).not.toMatch(/invoke\s*[<(]/);
		expect(VIEW).not.toContain("'music_");
		expect(VIEW).not.toContain('musicNowPlaying');
	});

	test('dibuja la tarjeta de la librería, no una propia', () => {
		expect(VIEW).toMatch(/import \{[^}]*NowPlayingCard[^}]*\} from '@vasakgroup\/vue-libvasak'/);
		expect(VIEW).toContain('<NowPlayingCard');
	});

	test('los botones siguen a lo que el reproductor permite', () => {
		expect(VIEW).toContain(':can-go-previous="musicInfo.canGoPrevious"');
		expect(VIEW).toContain(':can-go-next="musicInfo.canGoNext"');
		expect(VIEW).toContain(':can-seek="musicInfo.canSeek"');
		expect(VIEW).toContain(':can-play-pause="canPlayPause"');
	});

	test('la barra recibe la duración de MPRIS, que en cero no dibuja nada', () => {
		expect(VIEW).toContain(':duration="musicInfo.length"');
		expect(VIEW).toContain(':format="formatDuration"');
	});

	test('el disco distingue pausa de detenido', () => {
		expect(VIEW).toContain('playbackStateOf(musicInfo.value.status)');
		expect(VIEW).toContain(':state="state"');
	});

	test('la salida de audio es la del applet de audio', () => {
		expect(VIEW).toContain('getAudioDevices');
		expect(VIEW).toContain('setAudioDevice({ deviceId: device.id })');
		expect(VIEW).toContain("'audio-devices-changed'");
		// El grupo de opciones de la librería, el mismo del applet de audio:
		// trae el anuncio y las flechas (los prueba montados vue-libvasak).
		expect(VIEW).toContain('<OptionGroup');
		expect(VIEW).not.toContain('role="radio"');
		expect(VIEW).not.toContain('type="radio"');
	});

	test('el espacio del ecualizador no se dibuja mientras no exista', () => {
		// La ranura `footer` de la tarjeta sin contenido no se dibuja; ponerle
		// algo vacío dibujaría la línea y un hueco.
		expect(VIEW).not.toContain('#footer');
	});

	test('cada vez que vuelve a la vista pide lo que muestra', () => {
		expect(VIEW).toContain('@shown="refresh"');
	});
});

describe('una sola barra de progreso', () => {
	test('el widget usa la de la librería y no la suya', () => {
		expect(WIDGET).toContain('<SeekBar');
		expect(WIDGET).not.toContain('type="range"');
		expect(WIDGET).toContain(':seekable="musicInfo.canSeek"');
	});
});

describe('la portada del panel', () => {
	test('en pausa se congela en vez de volver a cero', () => {
		// Sacar `animate-spin` con la pausa hacía saltar la portada a su
		// posición inicial. Desde vue-libvasak 2.2.0 eso lo hace
		// `SpinningCover` (probado montado en la librería): acá queda que
		// reciba el estado de reproducción, que es lo que distingue pausa de
		// detenido.
		expect(CONTROL).toContain('<SpinningCover');
		expect(CONTROL).toContain(':state="state"');
		expect(CONTROL).toContain('playbackStateOf(musicInfo.value.status)');
	});

	test('el aro de progreso y el botón son los de la librería', () => {
		// Ni el `conic-gradient` con su máscara a mano ni un `<button>` propio.
		expect(CONTROL).toContain(':progress="musicInfo.length > 0 ? progress * 100 : null"');
		expect(CONTROL).toMatch(/<SpinningCover[^/]*\n\s*interactive\n/);
		expect(CONTROL).not.toContain('conic-gradient');
		expect(CONTROL).not.toMatch(/<button\b/);
		for (const catalog of [ES, EN]) {
			expect(typeof catalog.components.TrayMusicControl.progress).toBe('string');
		}
	});
});

describe('las clases de la librería llegan a la hoja de estilos', () => {
	test('Tailwind mira el paquete compilado', () => {
		expect(CSS).toContain('@source "../../node_modules/@vasakgroup/vue-libvasak/dist";');
	});
});

describe('los textos', () => {
	const KEYS = [
		'previous',
		'next',
		'play',
		'pause',
		'seek',
		'byArtist',
		'nothingPlaying',
		'currentOutput',
		'chooseOutput',
		'noOutputs',
		'via',
		'back',
	];

	test('cada texto del applet está en los dos idiomas', () => {
		for (const key of KEYS) {
			expect(typeof ES.views.musicApplet[key]).toBe('string');
			expect(typeof EN.views.musicApplet[key]).toBe('string');
			expect(VIEW).toContain(`'views.musicApplet.${key}'`);
		}
		expect(typeof ES.components.TrayMusicControl.openPlayer).toBe('string');
		expect(typeof EN.components.TrayMusicControl.openPlayer).toBe('string');
	});

	test('los que llevan un nombre adentro tienen dónde ponerlo', () => {
		for (const key of ['byArtist', 'currentOutput', 'via']) {
			expect(ES.views.musicApplet[key]).toContain('{0}');
			expect(EN.views.musicApplet[key]).toContain('{0}');
		}
	});
});
