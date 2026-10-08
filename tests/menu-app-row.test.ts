import { afterAll, beforeAll, describe, expect, test } from 'bun:test';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { createSSRApp, h } from 'vue';
import { compileScript, parse } from 'vue/compiler-sfc';
import { renderToString } from 'vue/server-renderer';

/**
 * En la pantalla principal del menú cada aplicación es una fila: el icono a la
 * izquierda y el nombre al lado, como era antes de la migración.
 *
 * Con 1.23.0 el icono y el nombre iban los dos en la ranura principal de
 * `DropdownMenuItem`, que es la **columna** del texto: el nombre caía debajo
 * del icono. Acá se compila el `AppMenuCard.vue` de verdad y se dibuja con el
 * ítem publicado de la librería, así que la prueba mira el HTML que sale y no
 * lo que dice el código. Y se dibuja también la plantilla de 1.23.0, para ver
 * que la prueba la rechaza.
 */
const SOURCE = join(import.meta.dir, '..', 'src', 'components', 'cards', 'AppMenuCard.vue');
const APP = { name: 'Firefox', icon: 'firefox', path: '/usr/share/applications/firefox.desktop', description: 'Navegador web' };

/** La plantilla de 1.23.0, la que dejaba el nombre debajo del icono. */
const TEMPLATE_123 = `<template>
  <DropdownMenuItem :title="app.description" class="w-full" @select="openApp">
    <ThemeIcon :name="app.icon" :size="32" alt="" />
    <span class="min-w-0 flex-1 break-words text-left">{{ app.name }}</span>
  </DropdownMenuItem>
</template>`;

let workdir = '';

async function render(source: string, name: string): Promise<string> {
	const { descriptor } = parse(source, { filename: SOURCE });
	const compiled = compileScript(descriptor, { id: name, inlineTemplate: true });
	// Los servicios del escritorio se cambian por dobles: si se importara el
	// logger de verdad, quedaría en la caché de módulos armado con el
	// `core.service` real, y la prueba del logger, que lo simula con
	// `mock.module`, recibiría ése según el orden en que corran los archivos.
	const code = compiled.content.replace(/from\s+(['"])@\/[^'"]+\1/g, "from './doubles'");
	const file = join(workdir, `${name}.ts`);
	writeFileSync(file, code);
	const component = (await import(file)).default;
	return renderToString(createSSRApp({ render: () => h(component, { app: APP }) }));
}

/** El ítem de menú, y lo que va en la ranura del costado y en la columna. */
function rowOf(html: string) {
	const item = html.match(/<div role="menuitem"[^>]*?class="([^"]*)"[^>]*>/);
	const side = html.match(/<span aria-hidden="true" class="flex shrink-0 items-center">([\s\S]*?)<\/span><span class="flex min-w-0 flex-1 flex-col">/);
	const columnStart = html.indexOf('<span class="flex min-w-0 flex-1 flex-col">');
	return {
		classes: (item?.[1] ?? '').split(/\s+/),
		side: side?.[1] ?? null,
		column: columnStart === -1 ? '' : html.slice(columnStart),
	};
}

/** Icono al costado y nombre en la columna, en una fila. */
function iconBesideName(html: string): boolean {
	const row = rowOf(html);
	return (
		row.classes.includes('flex') &&
		row.classes.includes('items-center') &&
		!row.classes.includes('flex-col') &&
		row.side !== null &&
		/<(img|span|div)\b/.test(row.side) &&
		!row.side.includes('Firefox') &&
		row.column.includes('Firefox') &&
		!/<img\b/.test(row.column)
	);
}

let current = '';
let old = '';

beforeAll(async () => {
	// Dentro del repositorio, para que `vue` y la librería se resuelvan desde
	// su `node_modules`.
	workdir = mkdtempSync(join(import.meta.dir, '..', 'src', '.menu-row-'));
	writeFileSync(
		join(workdir, 'doubles.ts'),
		[
			'export const openApp = async () => {};',
			'export const dismissMenu = async () => {};',
			'export const logError = () => {};',
			// Lanzar y fijar/desfijar salen del composable compartido (vasak-desktop#203).
			'export const useAppLauncher = () => ({ launch: async () => {}, toggleFavoriteFor: async () => {} });',
			'',
		].join('\n')
	);
	const source = readFileSync(SOURCE, 'utf8');
	current = await render(source, 'current');
	old = await render(source.replace(/<template>[\s\S]*<\/template>/, TEMPLATE_123), 'release-123');
});

afterAll(() => {
	if (workdir) rmSync(workdir, { recursive: true, force: true });
});

describe('la fila de una aplicación en el menú', () => {
	test('el icono va al costado y el nombre al lado, en una fila', () => {
		const row = rowOf(current);
		expect(row.classes).toContain('flex');
		expect(row.classes).toContain('items-center');
		expect(row.classes).not.toContain('flex-col');
		expect(row.side).not.toBeNull();
		expect(row.column).toContain('data-app-name');
		expect(iconBesideName(current)).toBe(true);
	});

	test('el icono va antes que el nombre', () => {
		expect(current.indexOf('flex shrink-0 items-center')).toBeGreaterThan(-1);
		expect(current.indexOf('flex shrink-0 items-center')).toBeLessThan(current.indexOf('Firefox'));
	});

	test('la plantilla de 1.23.0, con el nombre debajo del icono, no pasa', () => {
		expect(old).toContain('Firefox');
		expect(iconBesideName(old)).toBe(false);
	});
});
