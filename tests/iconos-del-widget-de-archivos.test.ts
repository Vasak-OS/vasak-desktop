/**
 * Los iconos del escritorio siguen al tema.
 *
 * No seguían. `file.controller.ts` resolvía el icono de cada archivo con
 * `getIconSource` y guardaba el resultado **en el dato** —`FileEntry.icon`—,
 * que el widget dibujaba con un `img` común. Un dato no se entera de que el
 * tema cambió, así que al pasar de claro a oscuro los iconos del widget se
 * quedaban con los del tema anterior hasta reabrir la ventana.
 *
 * Lo que sostenía esa resolución a mano era una rama que nunca corrió: pasar la
 * respuesta por `convertFileSrc` «cuando es una ruta absoluta». `getIconSource`
 * no devuelve rutas —arma el `data:` en `guest-js` con el base64 que manda el
 * backend, o devuelve la cadena vacía—, así que el `startsWith('/')` era
 * siempre falso y no había nada que `ThemeIcon` no pudiera hacer.
 *
 * Ahora `icon` lleva el **nombre** y lo dibuja `ThemeIcon`. Lo que se vigila
 * acá es que no vuelva a guardarse un icono ya resuelto: eso no falla ni avisa
 * —los iconos se ven bien hasta que alguien cambia el tema—, que es como pasó
 * desapercibido la primera vez.
 *
 * Se mira el texto y no se monta porque este repositorio todavía no tiene con
 * qué montar, igual que `controles-de-la-libreria.test.ts`.
 */

import { describe, expect, test } from 'bun:test';
import { getIconNameForFile } from '../src/tools/file.controller';

const raiz = new URL('../src/', import.meta.url).pathname;
const leer = (ruta: string) => Bun.file(raiz + ruta).text();

describe('el nombre del icono de un archivo', () => {
	test('una carpeta es una carpeta', () => {
		expect(getIconNameForFile('Documentos', true)).toBe('folder');
	});

	test('sale de la extensión', () => {
		expect(getIconNameForFile('foto.png', false)).toBe('image-x-generic');
		expect(getIconNameForFile('informe.pdf', false)).toBe('application-pdf');
	});

	test('la extensión en mayúsculas es la misma extensión', () => {
		// Un `.PNG` del celular o de una cámara es lo más común que hay.
		expect(getIconNameForFile('FOTO.PNG', false)).toBe('image-x-generic');
	});

	test('lo que no se reconoce cae en el genérico, no en vacío', () => {
		// Vacío dejaría a `ThemeIcon` pidiendo un nombre que no existe, y el
		// complemento devuelve el cuadrito de imagen rota sin decir que falló.
		expect(getIconNameForFile('algo.qwerty', false)).toBe('text-x-generic');
		expect(getIconNameForFile('sin-extension', false)).toBe('text-x-generic');
	});

	test('un archivo oculto no es su propia extensión', () => {
		// `'.bashrc'.split('.')` da dos partes y la segunda no es una extensión.
		expect(getIconNameForFile('.bashrc', false)).toBe('text-x-generic');
	});

	test('nunca devuelve algo que parezca ya resuelto', () => {
		const nombres = [
			getIconNameForFile('Documentos', true),
			getIconNameForFile('foto.png', false),
			getIconNameForFile('sin-extension', false),
		];

		for (const nombre of nombres) {
			expect(nombre).not.toStartWith('data:');
			expect(nombre).not.toStartWith('/');
			expect(nombre.length).toBeGreaterThan(0);
		}
	});
});

describe('quién dibuja el icono', () => {
	test('el widget lo pide por nombre a la librería', async () => {
		const widget = await leer('components/widgets/FilesWidget.vue');

		expect(widget).toContain("import { ThemeIcon } from '@vasakgroup/vue-libvasak'");
		expect(widget).toMatch(/<ThemeIcon\s+:name="file\.icon"/);
	});

	test('y no queda ningún `img` dibujando el dato', async () => {
		// El `img` que había es el que no seguía al tema. `previewUrl` —la
		// miniatura de una imagen o un video— no es un icono y no pasa por acá.
		const widget = await leer('components/widgets/FilesWidget.vue');

		expect(widget).not.toMatch(/:src="file\.icon"/);
	});

	test('el controlador no resuelve iconos', async () => {
		// La guardia de `lo-que-dibujaba-solo.test.ts` mira lo mismo desde el
		// otro lado —quién importa el complemento— y ya no lo tiene anotado
		// como excepción. Acá queda el porqué, al lado de lo que se probó.
		const controlador = await leer('tools/file.controller.ts');

		expect(controlador).not.toContain('getIconSource');
		expect(controlador).not.toContain('getFileIconSource');
	});

	test('el nombre se pone al armar la entrada, sin ir al backend', async () => {
		// Antes era una llamada por IPC **por archivo** al abrir el escritorio,
		// y otra tanda entera cada vez que el disco avisaba que algo cambió.
		// Ahora no hay ninguna: el nombre es una cuenta de la extensión, y
		// resolverlo queda del lado de `ThemeIcon`, que comparte la memoria
		// entre todos los archivos que usan el mismo icono.
		const controlador = await leer('tools/file.controller.ts');

		expect(controlador).toMatch(/icon: getIconNameForFile\(entry\.name, entry\.isDirectory\)/);
	});
});
