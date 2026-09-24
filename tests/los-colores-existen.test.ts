import { describe, expect, test } from 'bun:test';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Las clases de color que no están declaradas en el `@theme`.
 *
 * Tailwind v4 arma las utilidades a partir de las variables `--color-*` del
 * bloque `@theme`. Una clase que nombre un token que no está ahí **no emite
 * ninguna regla y no avisa**: no falla la compilación, no sale un aviso, y en
 * pantalla el elemento se queda con lo que herede. Se ve «casi bien», que es
 * por qué sobreviven años.
 *
 * El escritorio tenía 15 así. La más cara era `text-tx-primary`, en 101
 * lugares: el token es `tx-main`, y `tx-primary` no existió nunca, así que
 * todos esos textos venían heredando el color en vez de tomar el suyo. Después
 * `status-danger` en 35 —el declarado es `status-error`—, un `tx-link` y un
 * `ui-border-hover`.
 *
 * La prueba lee los tokens del CSS de verdad, no una lista escrita a mano: una
 * lista se queda vieja en cuanto alguien agrega un color, y entonces la prueba
 * empieza a mentir en la dirección cómoda.
 *
 * Sólo se juzgan los prefijos del taller. Un `bg-red-500` o un `text-white`
 * son de Tailwind y existen sin que nadie los declare.
 */

const NUESTROS = ['tx-', 'ui-', 'status-', 'primary', 'secondary', 'vsk-'];

const UTILIDADES = [
	'bg',
	'text',
	'border',
	'ring',
	'fill',
	'stroke',
	'from',
	'via',
	'to',
	'shadow',
	'outline',
	'decoration',
	'accent',
	'caret',
	'divide',
	'placeholder',
];

const raiz = join(import.meta.dir, '..');

/** Los tokens que el `@theme` declara, leídos del CSS. */
function tokensDeclarados(): Set<string> {
	const tokens = new Set<string>();

	for (const archivo of archivosCon(join(raiz, 'src'), ['.css'])) {
		const css = readFileSync(archivo, 'utf8');
		for (const m of css.matchAll(/--color-([a-z0-9-]+)\s*:/g)) {
			tokens.add(m[1]);
		}
	}

	return tokens;
}

function archivosCon(dir: string, extensiones: string[]): string[] {
	const salida: string[] = [];

	for (const entrada of readdirSync(dir)) {
		const ruta = join(dir, entrada);
		if (statSync(ruta).isDirectory()) {
			salida.push(...archivosCon(ruta, extensiones));
		} else if (extensiones.some((e) => entrada.endsWith(e))) {
			salida.push(ruta);
		}
	}

	return salida;
}

/**
 * Las clases de color usadas en el código, con el archivo donde están.
 *
 * El `(?![a-z0-9-])` es lo que separa `ui-border` de `ui-border-strong`: sin
 * ese borde, `border-ui-border-strong` se leería como el token `ui-border` —que
 * sí existe— y la clase muerta pasaría. Es el mismo agujero que una revisión ya
 * marcó en otra guardia de este taller.
 */
function clasesUsadas(): Map<string, Set<string>> {
	const patron = new RegExp(
		`\\b(?:${UTILIDADES.join('|')})-((?:${NUESTROS.join('|')})[a-z0-9-]*)(?![a-z0-9-])`,
		'g'
	);
	const encontradas = new Map<string, Set<string>>();

	for (const archivo of archivosCon(join(raiz, 'src'), ['.vue', '.ts', '.js'])) {
		const texto = readFileSync(archivo, 'utf8');
		for (const m of texto.matchAll(patron)) {
			const token = m[1];
			if (!encontradas.has(token)) encontradas.set(token, new Set());
			encontradas.get(token)?.add(archivo.slice(raiz.length + 1));
		}
	}

	return encontradas;
}

describe('las clases de color', () => {
	const declarados = tokensDeclarados();

	test('el CSS declara los tokens que se esperan, así que la prueba mira algo', () => {
		// Sin esto, un CSS que no se encuentre dejaría el conjunto vacío y todo
		// lo de abajo fallaría por el motivo equivocado —o peor, si se invirtiera
		// la comparación, pasaría siempre—.
		expect(declarados.size).toBeGreaterThan(10);
		for (const esperado of ['tx-main', 'tx-muted', 'ui-surface', 'status-error', 'primary']) {
			expect(declarados, `falta --color-${esperado}`).toContain(esperado);
		}
	});

	test('todas las que se usan están declaradas en el @theme', () => {
		const muertas: string[] = [];

		for (const [token, archivos] of clasesUsadas()) {
			if (!declarados.has(token)) {
				muertas.push(`${token} (${[...archivos].slice(0, 3).join(', ')})`);
			}
		}

		expect(muertas).toEqual([]);
	});

	test('y los tokens que ya no existen no vuelven por su nombre viejo', () => {
		// Los dos que había. Van por nombre además de por la regla general
		// porque el mensaje de arriba dice «no está declarado» y éste dice cuál
		// es el reemplazo, que es lo que hace falta cuando reaparecen.
		const reemplazos: Record<string, string> = {
			'vsk-text': 'tx-main',
			'ui-main': 'tx-main',
		};

		const usadas = clasesUsadas();
		for (const [viejo, nuevo] of Object.entries(reemplazos)) {
			expect(usadas.has(viejo), `«${viejo}» no existe: va «${nuevo}»`).toBe(false);
		}
	});
});
