import { describe, expect, it } from 'bun:test';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Que toda clave citada en el código exista en el catálogo, y que los controles
 * que sólo tienen un icono reciban su etiqueta.
 *
 * Este test nace de haberle puesto nombre accesible a los controles: cada
 * `label` es un `t('...')` nuevo, y una clave mal escrita no falla en ninguna
 * parte —`t()` devuelve la clave cruda y el lector de pantalla anuncia
 * «views.power.idleLok»—. En un texto visible se nota; en el nombre de un
 * interruptor, que existe justamente para quien no ve la pantalla, no.
 */
const RAIZ_LOCALES = join(import.meta.dir, '../../src-tauri/locales');
const RAIZ_FUENTE = join(import.meta.dir, '../../src');

/** Controles sin texto propio: el nombre les tiene que llegar por prop. */
const CON_ETIQUETA_OBLIGATORIA = ['SwitchToggle', 'ToggleControl', 'SliderControl', 'CategoryMenuPill'] as const;

const catalogo = () =>
	Bun.YAML.parse(readFileSync(join(RAIZ_LOCALES, 'es.yml'), 'utf8')) as Record<string, unknown>;

function archivos(directorio: string): string[] {
	return readdirSync(directorio).flatMap((entrada) => {
		const ruta = join(directorio, entrada);
		if (statSync(ruta).isDirectory()) return archivos(ruta);
		return ruta.endsWith('.vue') ? [ruta] : [];
	});
}

function existe(documento: Record<string, unknown>, clave: string) {
	const valor = clave.split('.').reduce<any>((nodo, parte) => nodo?.[parte], documento);
	return typeof valor === 'string' || typeof valor === 'number';
}

/**
 * La etiqueta de apertura que arranca en `desde`. No sirve una expresión
 * regular: los atributos llevan valores con `>` adentro —`(v) => algo`— y
 * cortar en el primer `>` parte la etiqueta al medio.
 */
function etiquetaDesde(texto: string, desde: number) {
	let comilla = '';
	for (let i = desde; i < texto.length; i++) {
		const c = texto[i];
		if (comilla) {
			if (c === comilla) comilla = '';
		} else if (c === '"' || c === "'") {
			comilla = c;
		} else if (c === '>') {
			return texto.slice(desde, i + 1);
		}
	}
	return texto.slice(desde);
}

describe('etiquetas de la interfaz', () => {
	it('toda clave citada existe en el catálogo', () => {
		const documento = catalogo();
		const faltantes: string[] = [];

		for (const ruta of archivos(RAIZ_FUENTE)) {
			const texto = readFileSync(ruta, 'utf8');
			// Sólo las literales: las armadas con plantilla
			// (`wayfire.plugins.\${id}.label`) dependen de datos externos.
			for (const coincidencia of texto.matchAll(/\bt\(\s*'([a-zA-Z][\w.]*)'\s*\)/g)) {
				if (!existe(documento, coincidencia[1])) {
					faltantes.push(`${ruta.split('/src/')[1]} → ${coincidencia[1]}`);
				}
			}
		}

		expect(faltantes).toEqual([]);
	});

	it('cada control sin texto propio recibe su etiqueta', () => {
		// El typecheck ya lo exige, pero sólo mientras la prop siga siendo
		// obligatoria: alcanza con que alguien le ponga un valor por omisión para
		// «desbloquear» una vista y los controles se quedan sin nombre en silencio.
		const sinEtiqueta: string[] = [];

		for (const ruta of archivos(RAIZ_FUENTE)) {
			const texto = readFileSync(ruta, 'utf8');
			for (const componente of CON_ETIQUETA_OBLIGATORIA) {
				const marca = `<${componente}`;
				for (let i = texto.indexOf(marca); i !== -1; i = texto.indexOf(marca, i + 1)) {
					// Que no sea el prefijo de otro componente más largo.
					if (/[\w-]/.test(texto[i + marca.length] ?? '')) continue;
					const etiqueta = etiquetaDesde(texto, i);
					if (!/[\s:]label\s*=/.test(etiqueta)) {
						sinEtiqueta.push(`${ruta.split('/src/')[1]} → <${componente}>`);
					}
				}
			}
		}

		expect(sinEtiqueta).toEqual([]);
	});
});
