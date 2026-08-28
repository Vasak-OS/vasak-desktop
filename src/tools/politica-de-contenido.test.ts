import { describe, expect, test } from 'bun:test';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * La política de contenido y el código tienen que decir lo mismo.
 *
 * Un servidor que el código consulta pero la política no nombra no falla al
 * compilar ni al arrancar: el webview corta el pedido en silencio y lo único
 * que se ve es el widget diciendo que no pudo. Fue exactamente lo que le pasó
 * al clima, que geocodifica contra `geocoding-api.open-meteo.com` mientras la
 * política sólo permitía `api.open-meteo.com`. Este test compara las dos listas
 * para que la próxima vez el que se olvide sea el test y no quien usa el
 * escritorio.
 */

const RAIZ = join(import.meta.dir, '..');
const CONFIGURACION = join(RAIZ, '..', 'src-tauri', 'tauri.conf.json');

/** Los servidores que `connect-src` deja consultar. */
function permitidos(): Set<string> {
	const { app } = JSON.parse(readFileSync(CONFIGURACION, 'utf8'));
	const csp: string = app.security.csp;

	const directiva = csp
		.split(';')
		.map((parte) => parte.trim())
		.find((parte) => parte.startsWith('connect-src'));

	if (!directiva) throw new Error('La política no tiene connect-src');

	return new Set(
		directiva
			.split(/\s+/)
			.slice(1)
			.filter((origen) => origen.startsWith('https://'))
			.map((origen) => origen.replace('https://', ''))
	);
}

/** Los servidores que el código nombra, con dónde los nombra. */
function usados(): Map<string, string[]> {
	const encontrados = new Map<string, string[]>();

	const recorrer = (carpeta: string) => {
		for (const entrada of readdirSync(carpeta, { withFileTypes: true })) {
			const camino = join(carpeta, entrada.name);

			if (entrada.isDirectory()) {
				recorrer(camino);
				continue;
			}
			if (!/\.(ts|vue)$/.test(entrada.name) || entrada.name.endsWith('.test.ts')) continue;

			// Un servidor vacío —el `https://` suelto de un comentario o de un
			// `startsWith`— no es nadie a quien se le pida nada.
			for (const [, servidor] of readFileSync(camino, 'utf8').matchAll(
				/https:\/\/([a-zA-Z0-9._-]+)/g
			)) {
				encontrados.set(servidor, [...(encontrados.get(servidor) ?? []), camino]);
			}
		}
	};

	recorrer(RAIZ);
	return encontrados;
}

describe('política de contenido', () => {
	test('permite todos los servidores que el código consulta', () => {
		const lista = permitidos();
		const faltantes = [...usados()]
			.filter(([servidor]) => !lista.has(servidor))
			.map(([servidor, archivos]) => `${servidor} (en ${archivos.join(', ')})`);

		expect(faltantes).toEqual([]);
	});

	test('no permite servidores que nadie consulta', () => {
		const lista = usados();
		const sobrantes = [...permitidos()].filter((servidor) => !lista.has(servidor));

		expect(sobrantes).toEqual([]);
	});
});
