import { describe, expect, test } from 'bun:test';
import { MARCADORES_CSP, sanearUrl } from '../src/tools/csp';

/**
 * Lo que se registra al bloquearse un recurso es una URL que eligió otro, y las
 * URL llevan credenciales. Lo que estas pruebas cuidan es que ninguna termine
 * en el diario.
 */
describe('sanearUrl', () => {
	test('una URL relativa al protocolo no deja pasar las credenciales', () => {
		// El caso reportado: `new URL` sin base rechaza estas direcciones, y la
		// versión anterior caía a cortar por `?` y `#`, que deja el token.
		const limpia = sanearUrl('//user:token@example.test/path?access_token=secret#frag');

		for (const secreto of ['user', 'token', 'access_token', 'secret', 'frag']) {
			expect(limpia).not.toContain(secreto);
		}
		expect(limpia).toBe('//example.test/path');
	});

	test('y sin credenciales conserva el sitio y la ruta', () => {
		expect(sanearUrl('//example.test/path')).toBe('//example.test/path');
		expect(sanearUrl('//example.test/a/b.js?x=1')).toBe('//example.test/a/b.js');
	});

	/**
	 * El agujero que no estaba en el reporte y se encontró midiendo.
	 *
	 * `new URL('user:token@sitio/x')` **no** falla: lo toma como esquema opaco
	 * `user:`, deja `username` vacío —así que limpiarlo no hace nada— y devuelve
	 * el token entero en `href`. O sea que también filtraba por la rama buena.
	 */
	test('un esquema opaco no deja pasar lo que lleva adentro', () => {
		const limpia = sanearUrl('user:token@example.test/path');
		expect(limpia).not.toContain('token');
		expect(limpia).toBe('user:(recortado)');
	});

	test('las credenciales de una URL absoluta tampoco', () => {
		const limpia = sanearUrl('https://user:token@example.test/p?x=1#f');
		for (const secreto of ['user', 'token', 'x=1', '#f']) {
			expect(limpia).not.toContain(secreto);
		}
		expect(limpia).toBe('https://example.test/p');
	});

	test('los marcadores de la especificación se dejan tal cual', () => {
		for (const marcador of MARCADORES_CSP) {
			expect(sanearUrl(marcador)).toBe(marcador);
		}
	});

	/**
	 * `data` sin dos puntos es un marcador y se conserva; `data:` con contenido
	 * es una carga útil y se recorta. Son dos cosas distintas que se escriben
	 * casi igual.
	 */
	test('una URL de datos se recorta pero se sigue sabiendo que lo era', () => {
		expect(sanearUrl('data:text/html;base64,UEFTUw==')).toBe('data:(recortado)');
		expect(sanearUrl('data')).toBe('data');
	});

	test('una ruta relativa pierde la query y el fragmento', () => {
		expect(sanearUrl('/assets/app.js?v=2#top')).toBe('/assets/app.js');
		expect(sanearUrl('app.js')).toBe('app.js');
	});

	/**
	 * Vacío al entrar y vacío al salir son casos distintos, y quien llame tiene
	 * que aplicar su texto de reserva **después** de sanear. Una entrada como
	 * `?token=X` no es vacía, pero lo que queda de ella sí — y sin esto el
	 * registro salía con el campo en blanco.
	 */
	test('lo que queda en nada devuelve vacío', () => {
		expect(sanearUrl('?token=X')).toBe('');
		expect(sanearUrl('#fragment')).toBe('');
		expect(sanearUrl('')).toBe('');
		expect(sanearUrl(null)).toBe('');
		expect(sanearUrl(undefined)).toBe('');
	});

	test('nunca devuelve algo que parezca credencial', () => {
		// Una red de seguridad sobre todos los casos de arriba juntos: si en el
		// resultado queda un `@` antes de la primera barra, hay userinfo.
		for (const entrada of [
			'//u:p@sitio/x',
			'https://u:p@sitio/x',
			'user:p@sitio/x',
			'ftp://u:p@sitio/x',
		]) {
			const limpia = sanearUrl(entrada);
			const autoridad = limpia.replace(/^[a-z]+:/, '').replace(/^\/\//, '').split('/')[0];
			expect(autoridad).not.toContain('@');
		}
	});
});
