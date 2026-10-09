import { describe, expect, test } from 'bun:test';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Las bibliotecas que se quedan atrás a propósito.
 *
 * `glib` va en la 0.18 porque la fija gtk-rs a través de tauri, y Dependabot
 * marca una falla en `VariantStrIter` que se corrige recién en la 0.20. Mientras
 * siga así, la decisión tiene que estar escrita en `vasak.bibliotecasAtrasadas`,
 * y el código propio no puede empezar a usar la API afectada: la nota dice que
 * sólo la tocan las dependencias, y eso deja de ser cierto sin que nada falle.
 */

const root = join(import.meta.dir, '..');
const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
const lock = readFileSync(join(root, 'src-tauri/Cargo.lock'), 'utf8');

function rustFiles(dir: string): string[] {
	return readdirSync(dir).flatMap((name) => {
		const path = join(dir, name);
		if (statSync(path).isDirectory()) return rustFiles(path);
		return name.endsWith('.rs') ? [path] : [];
	});
}

const glibVersions = [...lock.matchAll(/name = "glib"\nversion = "(\d+)\.(\d+)\.\d+"/g)].map(
	([, major, minor]) => Number(major) * 1000 + Number(minor)
);
const glibIsVulnerable = glibVersions.some((v) => v < 20);

describe('glib atrasada', () => {
	test('el bloqueo tiene glib, y la cuenta sirve', () => {
		expect(glibVersions.length).toBeGreaterThan(0);
	});

	test.if(glibIsVulnerable)('mientras sea < 0.20, la nota explica por qué y cuándo se saca', () => {
		const note: string | undefined = pkg.vasak?.bibliotecasAtrasadas?.['glib (crate de Rust)'];
		expect(note).toBeString();
		expect(note).toContain('VariantStrIter');
		expect(note).toContain('Se saca cuando');
	});

	test.if(glibIsVulnerable)('el código propio no usa la API afectada', () => {
		const uses = rustFiles(join(root, 'src-tauri/src')).filter((file) =>
			/VariantStrIter|array_iter_str/.test(readFileSync(file, 'utf8'))
		);
		expect(uses).toEqual([]);
	});

	test.if(!glibIsVulnerable)('con glib 0.20 o posterior, la nota ya no hace falta', () => {
		expect(pkg.vasak?.bibliotecasAtrasadas?.['glib (crate de Rust)']).toBeUndefined();
	});
});
