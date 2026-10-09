/**
 * Las cuentas del widget de música (vasak-desktop#166).
 *
 * Lo que el widget deriva del estado de MPRIS vive en `tools/music-widget.ts`
 * para poder probarse sin montar el componente.
 */
import { describe, expect, test } from 'bun:test';
import type { MusicInfo } from '../src/interfaces/music';
import {
	coverRingProgress,
	hasExtras,
	loopStatusIconName,
	percentToVolume,
	playPauseAvailable,
	seekRatio,
	volumeToPercent,
} from '../src/tools/music-widget';

function info(overrides: Partial<MusicInfo> = {}): MusicInfo {
	return {
		player: 'org.mpris.MediaPlayer2.vlc',
		playerIdentity: 'VLC',
		desktopEntry: 'vlc',
		status: 'Playing',
		title: 'Tema',
		artist: 'Artista',
		album: 'Disco',
		artUrl: '',
		length: 200_000_000,
		position: 50_000_000,
		trackId: '1',
		canGoNext: true,
		canGoPrevious: true,
		canPlay: true,
		canPause: true,
		canSeek: true,
		canControl: true,
		canRaise: true,
		shuffle: null,
		loopStatus: null,
		volume: null,
		...overrides,
	};
}

describe('reproducir o pausar según lo que el reproductor acepta', () => {
	test('sin reproductor, no se puede', () => {
		expect(playPauseAvailable(info({ player: '' }), false)).toBe(false);
	});
	test('sonando, mira si se puede pausar', () => {
		expect(playPauseAvailable(info({ canPause: true }), true)).toBe(true);
		expect(playPauseAvailable(info({ canPause: false }), true)).toBe(false);
	});
	test('en pausa, mira si se puede reproducir', () => {
		expect(playPauseAvailable(info({ canPlay: true }), false)).toBe(true);
		expect(playPauseAvailable(info({ canPlay: false }), false)).toBe(false);
	});
});

describe('el icono de repetición', () => {
	test('el de una pista cuando repite la pista', () => {
		expect(loopStatusIconName('Track', 'loop', 'loop-one')).toBe('loop-one');
	});
	test('el normal en los demás casos', () => {
		expect(loopStatusIconName('Playlist', 'loop', 'loop-one')).toBe('loop');
		expect(loopStatusIconName('None', 'loop', 'loop-one')).toBe('loop');
		expect(loopStatusIconName(null, 'loop', 'loop-one')).toBe('loop');
	});
});

describe('si hay extras que mostrar', () => {
	test('ninguno cuando el reproductor no implementa nada de eso', () => {
		expect(hasExtras(info({ shuffle: null, loopStatus: null, volume: null }))).toBe(false);
	});
	test('alcanza con que exista uno', () => {
		expect(hasExtras(info({ shuffle: false }))).toBe(true);
		expect(hasExtras(info({ loopStatus: 'None' }))).toBe(true);
		expect(hasExtras(info({ volume: 0 }))).toBe(true);
	});
});

describe('el aro de avance del disco', () => {
	test('la fracción en 0–100 cuando hay duración', () => {
		expect(coverRingProgress(50_000_000, 200_000_000)).toBe(25);
	});
	test('sin duración, no hay aro', () => {
		expect(coverRingProgress(10, 0)).toBeNull();
	});
});

describe('el volumen entre 0–1 y 0–100', () => {
	test('a porcentaje, redondeado; null es cero', () => {
		expect(volumeToPercent(0.7)).toBe(70);
		expect(volumeToPercent(null)).toBe(0);
	});
	test('de vuelta a 0–1, acotado', () => {
		expect(percentToVolume(70)).toBeCloseTo(0.7, 5);
		expect(percentToVolume(150)).toBe(1);
		expect(percentToVolume(-10)).toBe(0);
	});
});

describe('la fracción para un salto', () => {
	test('la posición sobre la duración', () => {
		expect(seekRatio(100_000_000, 200_000_000)).toBeCloseTo(0.5, 5);
	});
	test('sin duración, no se puede saltar', () => {
		expect(seekRatio(10, 0)).toBeNull();
	});
});
