import { describe, expect, test } from 'bun:test';
import { playbackStateOf } from './playback';

describe('el estado que dibuja el disco', () => {
	test('lo que manda MPRIS, en cualquier caja', () => {
		expect(playbackStateOf('Playing')).toBe('playing');
		expect(playbackStateOf('paused')).toBe('paused');
		expect(playbackStateOf('Stopped')).toBe('stopped');
	});

	test('lo que no se entiende es detenido: un disco quieto no promete nada', () => {
		expect(playbackStateOf('')).toBe('stopped');
		expect(playbackStateOf(undefined)).toBe('stopped');
		expect(playbackStateOf(null)).toBe('stopped');
		expect(playbackStateOf('Buffering')).toBe('stopped');
	});
});
