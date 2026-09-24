/**
 * De dónde puede salir una imagen del escritorio.
 *
 * La carátula que publica el reproductor es la única imagen que el escritorio
 * no elige: la ruta —o la URL— la escribe otro proceso. Por eso las dos reglas
 * que la dejan entrar están acá escritas como prueba y no sólo en la
 * configuración, donde cambiarlas no cuesta nada y no avisa nada.
 */
import { describe, expect, test } from 'bun:test';
import config from '../src-tauri/tauri.conf.json';

const csp: string = config.app.security.csp;
const imgSrc = csp.split(';').find((d) => d.trim().startsWith('img-src')) ?? '';
const alcance: string[] = config.app.security.assetProtocol.scope;

describe('la política de contenido y la carátula', () => {
	/**
	 * Los bytes llegan por el IPC y el frontend arma un `blob:`. Sin esto, la
	 * carátula de cualquier reproductor que la deje fuera del alcance declarado
	 * —Chromium la deja en `/tmp`— no se ve, y no falla: cae al icono genérico,
	 * que se ve igual que «este reproductor no manda carátula».
	 */
	test('un blob propio puede dibujarse', () => {
		expect(imgSrc).toContain('blob:');
	});

	/**
	 * Spotify y los demás que publican la carátula como URL. Va `https:` y no
	 * `http:` a propósito: si el escritorio va a pedirle una imagen a un
	 * servidor que eligió otro, que sea por un canal cifrado.
	 */
	test('una carátula remota se pide cifrada, o no se pide', () => {
		expect(imgSrc).toContain('https:');

		// `http://asset.localhost` es el protocolo de archivos de Tauri hablando
		// consigo mismo, no una dirección de internet: no sale de la máquina.
		const sinCifrar = imgSrc
			.split(/\s+/)
			.filter((fuente) => fuente.startsWith('http://') && !fuente.includes('asset.localhost'));
		expect(sinCifrar).toEqual([]);
	});

	/**
	 * La salida que **no** se tomó: agrandar el alcance del protocolo de assets
	 * hasta que entre `/tmp` le daría al WebView lectura de un directorio que es
	 * de todos, y cuyos nombres de archivo los elige otro proceso.
	 */
	test('el protocolo de assets no llega a los directorios temporales', () => {
		for (const ruta of alcance) {
			expect(ruta.startsWith('/tmp')).toBe(false);
			expect(ruta.startsWith('/var/tmp')).toBe(false);
		}
	});

	/** Y lo que ya estaba: nada de scripts ni de marcos de afuera. */
	test('lo que se ejecuta sigue siendo sólo lo propio', () => {
		expect(csp).toContain("script-src 'self'");
		expect(csp).toContain("frame-src 'none'");
		expect(csp).toContain("object-src 'none'");
	});
});
