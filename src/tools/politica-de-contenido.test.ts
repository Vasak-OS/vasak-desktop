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

/**
 * El servidor de un origen, con su puerto si no es el de siempre.
 *
 * Las dos listas tienen que normalizarse igual o la comparación miente en las
 * dos direcciones: un `:8443` que una lista guarda y la otra descarta hace que
 * un servidor permitido parezca faltante, y —peor— que un `https://donde:8443`
 * del código parezca cubierto por un `https://donde` de la política, que en
 * realidad no lo cubre: la política no acepta un puerto que su origen no
 * nombra.
 */
function normalizarOrigen(origen: string): string {
	return new URL(origen).host;
}

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
			.map(normalizarOrigen)
	);
}

/**
 * El archivo sin sus comentarios.
 *
 * Un comentario no consulta nada, y contarlo como si lo hiciera es lo que dejó
 * este test fallando en `main`: el ejemplo `blob:https://usuario:token@sitio/x`
 * que explica por qué `blob:` queda afuera en `csp.ts` daba el «servidor»
 * `usuario` —la parte de usuario de una URL con credenciales—, y la política
 * lógicamente no nombra a nadie así. El problema es de la clase que vuelve:
 * cualquier enlace a documentación en un comentario habría hecho lo mismo.
 *
 * No alcanza con borrar de `//` hasta el fin de la línea: el `//` de `https://`
 * abriría un comentario en el medio de las URL de verdad y se llevaría puesto
 * justo lo que este test busca —y el test pasaría, porque no encontrar nada no
 * es encontrar algo que falte—. Así que se recorre el archivo con las cadenas
 * como estado propio, y sólo fuera de ellas un `//` o un `/*` empieza un
 * comentario.
 *
 * Y una comilla simple o doble abre cadena sólo si cierra en su misma línea, que
 * es lo único que la especificación permite. Así la apóstrofe del texto de una
 * plantilla —`don't`— no abre nada: si abriera, el resto de la línea se
 * copiaría tal cual, y un `<!-- … -->` que viniera después en esa línea
 * quedaría sin filtrar. No alcanzaba con cortar la cadena en el fin de línea,
 * porque lo copiado hasta ahí es justamente lo que había que sacar.
 *
 * Lo quitado se reemplaza por espacios y saltos en lugar de sacarse, para no
 * pegar dos trozos que estaban separados.
 */
export function sinComentarios(fuente: string): string {
	let salida = '';
	let i = 0;

	const hueco = (c: string) => (c === '\n' ? '\n' : ' ');

	/** Si hay una comilla igual —sin escapar— antes de que termine la línea. */
	const cierraEnLaLinea = (desde: number, comilla: string) => {
		for (let j = desde; j < fuente.length; j++) {
			if (fuente[j] === '\\') {
				j++;
				continue;
			}
			if (fuente[j] === '\n') return false;
			if (fuente[j] === comilla) return true;
		}
		return false;
	};

	while (i < fuente.length) {
		const dos = fuente.slice(i, i + 2);

		if (dos === '//') {
			while (i < fuente.length && fuente[i] !== '\n') {
				salida += ' ';
				i++;
			}
			continue;
		}

		if (dos === '/*') {
			while (i < fuente.length && fuente.slice(i, i + 2) !== '*/') {
				salida += hueco(fuente[i]);
				i++;
			}
			salida += '  ';
			i += 2;
			continue;
		}

		// Los de las plantillas de Vue, que también son comentarios y también
		// pueden llevar una dirección de ejemplo.
		if (fuente.startsWith('<!--', i)) {
			while (i < fuente.length && !fuente.startsWith('-->', i)) {
				salida += hueco(fuente[i]);
				i++;
			}
			salida += '   ';
			i += 3;
			continue;
		}

		const comilla = fuente[i];

		if (comilla === '"' || comilla === "'" || comilla === '`') {
			// Sin pareja no es una cadena: es una apóstrofe del texto de una
			// plantilla. Se copia y se sigue mirando lo que viene, que puede ser
			// un comentario. La pareja se busca en la línea para las comillas y
			// en el resto del archivo para las plantillas, que sí cruzan líneas.
			const pareja =
				comilla === '`' ? fuente.indexOf('`', i + 1) !== -1 : cierraEnLaLinea(i + 1, comilla);

			if (!pareja) {
				salida += comilla;
				i++;
				continue;
			}

			salida += comilla;
			i++;

			while (i < fuente.length) {
				if (fuente[i] === '\\') {
					salida += fuente.slice(i, i + 2);
					i += 2;
					continue;
				}

				salida += fuente[i];
				i++;

				if (fuente[i - 1] === comilla) break;
			}
			continue;
		}

		salida += fuente[i];
		i++;
	}

	return salida;
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
			for (const [origen] of sinComentarios(readFileSync(camino, 'utf8')).matchAll(
				/https:\/\/[a-zA-Z0-9._-]+(?::\d+)?/g
			)) {
				const servidor = normalizarOrigen(origen);
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

	test('un puerto distinto es un servidor distinto', () => {
		// El puerto de siempre no se escribe, así que las dos formas de nombrar
		// al mismo servidor tienen que dar lo mismo.
		expect(normalizarOrigen('https://donde:443')).toBe(normalizarOrigen('https://donde'));
		// Y uno distinto sí, porque la política tampoco lo da por permitido.
		expect(normalizarOrigen('https://donde:8443')).not.toBe(normalizarOrigen('https://donde'));
	});

	test('los servidores que el código sí consulta se encuentran', () => {
		// El guardia del arreglo: quitar comentarios de más dejaría este test sin
		// nada que comparar, y sin nada que comparar nunca falta nada. Los dos
		// del clima están en plantillas de `useWeather`, o sea en código.
		const lista = usados();

		expect([...lista.keys()].sort()).toEqual([
			'api.open-meteo.com',
			'geocoding-api.open-meteo.com',
		]);
	});

	test('una dirección en un comentario no es un servidor que se consulte', () => {
		// El caso que dejó el test fallando: el ejemplo de `csp.ts`, que ni es una
		// dirección a la que se le pida algo ni tiene un servidor llamado
		// «usuario» —eso es la parte de usuario de una URL con credenciales—.
		const fuente = [
			'/**',
			' * `blob:https://usuario:token@sitio/x`',
			' */',
			"const url = 'https://api.open-meteo.com/v1/forecast';",
			'// https://developer.mozilla.org/docs',
		].join('\n');

		const encontrados = [
			...sinComentarios(fuente).matchAll(/https:\/\/[a-zA-Z0-9._-]+(?::\d+)?/g),
		].map(([origen]) => origen);

		expect(encontrados).toEqual(['https://api.open-meteo.com']);
	});

	test('el `//` de una dirección no abre un comentario', () => {
		// Lo que rompe la versión ingenua: borrar de `//` al fin de línea se lleva
		// la dirección de verdad, el test se queda sin nada y pasa igual.
		const fuente = "fetch('https://api.open-meteo.com/v1/forecast'); // el clima";

		expect(sinComentarios(fuente)).toContain('https://api.open-meteo.com/v1/forecast');
		expect(sinComentarios(fuente)).not.toContain('el clima');
	});

	test('una apóstrofe en una plantilla no se come el resto del archivo', () => {
		// Las cadenas de comillas simples y dobles terminan con la línea, como
		// dice la especificación. Si no, el `'` de `don't` abriría una cadena que
		// seguiría hasta la próxima comilla del archivo y escondería lo de abajo.
		const fuente = ["<span>don't</span>", "fetch('https://api.open-meteo.com/x');"].join('\n');

		expect(sinComentarios(fuente)).toContain('https://api.open-meteo.com/x');
	});

	test('una apóstrofe de texto no le abre la puerta a un comentario sin filtrar', () => {
		// Lo que encontró la revisión de #67: en una plantilla, la apóstrofe de
		// `don't` no tiene pareja, así que si abriera una cadena el resto de la
		// línea se copiaría tal cual —comentario incluido— y `usuario` volvería a
		// aparecer como servidor. Una cadena de verdad cierra en su misma línea;
		// una apóstrofe de texto, no.
		const fuente = "<span>don't</span> <!-- blob:https://usuario:token@sitio/x -->";

		const encontrados = [
			...sinComentarios(fuente).matchAll(/https:\/\/[a-zA-Z0-9._-]+(?::\d+)?/g),
		].map(([origen]) => origen);

		expect(encontrados).toEqual([]);
	});

	test('no permite servidores que nadie consulta', () => {
		const lista = usados();
		const sobrantes = [...permitidos()].filter((servidor) => !lista.has(servidor));

		expect(sobrantes).toEqual([]);
	});
});
