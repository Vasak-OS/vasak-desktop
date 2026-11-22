/**
 * La capability del chat de Lince es la única que dibuja texto del modelo
 * (entrada no confiable): tiene que seguir acotada, y la ventana `assistant`
 * no puede volver a la capability general.
 */
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const read = (name: string) =>
	JSON.parse(readFileSync(join(import.meta.dir, '..', 'src-tauri', 'capabilities', name), 'utf8')) as {
		windows: string[];
		permissions: Array<string | { identifier: string }>;
	};

const ids = (permissions: Array<string | { identifier: string }>) =>
	permissions.map((p) => (typeof p === 'string' ? p : p.identifier));

describe('la capability del asistente', () => {
	const assistant = read('assistant.json');

	test('es sólo de la ventana `assistant`', () => {
		expect(assistant.windows).toEqual(['assistant']);
	});

	test('no puede lanzar procesos, tocar archivos ni abrir enlaces ni usar los gestores de red', () => {
		const list = ids(assistant.permissions);
		const prohibido = /^(shell:|fs:|opener:|http:|bluetooth|network|drag|user-data)/;
		expect(list.filter((id) => prohibido.test(id))).toEqual([]);
		expect(list.some((id) => id.includes('allow-spawn') || id.includes('scope'))).toBe(false);
	});

	test('la ventana `assistant` no está en la capability general', () => {
		const general = read('default.json');
		expect(general.windows).not.toContain('assistant');
		expect(general.windows.some((w) => w === 'assistant*' || w === '*')).toBe(false);
	});
});
