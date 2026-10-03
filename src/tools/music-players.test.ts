import { describe, expect, test } from 'bun:test';
import type { PlayerRef } from '@/interfaces/music';
import { activePlayerIndex, playerLabels } from './music-players';

const ref = (player: string, extra: Partial<PlayerRef> = {}): PlayerRef => ({
	player,
	identity: '',
	status: 'Paused',
	title: '',
	active: false,
	pinned: false,
	...extra,
});

describe('cuál de los reproductores se está mostrando', () => {
	const players = [
		ref('org.mpris.MediaPlayer2.firefox.instance_1', { active: true }),
		ref('org.mpris.MediaPlayer2.vlc'),
	];

	test('manda el que llegó en el último aviso, no la marca vieja de la lista', () => {
		expect(activePlayerIndex(players, 'org.mpris.MediaPlayer2.vlc')).toBe(1);
	});

	test('si el aviso todavía no llegó, el marcado', () => {
		expect(activePlayerIndex(players, '')).toBe(0);
	});

	test('sin marcado, el primero; sin lista, cero', () => {
		expect(activePlayerIndex([ref('a'), ref('b')], 'c')).toBe(0);
		expect(activePlayerIndex([], 'c')).toBe(0);
	});
});

describe('cómo se llama cada punto', () => {
	test('la aplicación y lo que suena, que separa dos pestañas del mismo navegador', () => {
		expect(
			playerLabels([
				ref('org.mpris.MediaPlayer2.firefox.instance_1', {
					identity: 'Firefox',
					title: 'Un video',
				}),
				ref('org.mpris.MediaPlayer2.firefox.instance_2', { identity: 'Firefox', title: 'Otro' }),
			])
		).toEqual(['Firefox: Un video', 'Firefox: Otro']);
	});

	test('sin nombre, el del bus sin el prefijo de MPRIS', () => {
		expect(playerLabels([ref('org.mpris.MediaPlayer2.spotify')])).toEqual(['spotify']);
	});
});
