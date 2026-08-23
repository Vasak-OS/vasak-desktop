import { describe, expect, it } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Los dos archivos de idioma, revisados de verdad.
 *
 * Estos tests existen porque rompí las traducciones dos veces del mismo modo:
 * un valor sin comillas que contiene `": "` —`Transferencia de red: lo que
 * baja`— no es una cadena para YAML sino un mapa anidado mal formado, y el
 * archivo entero deja de cargar. La aplicación no avisa: se queda mostrando las
 * claves crudas. Un `grep` no lo ve; un parser sí.
 */
const RAIZ = join(import.meta.dir, '../../src-tauri/locales');
const IDIOMAS = ['es', 'en'] as const;

const cargar = (idioma: string) =>
	Bun.YAML.parse(readFileSync(join(RAIZ, `${idioma}.yml`), 'utf8'));

/** Todas las rutas hoja, para poder comparar los idiomas clave por clave. */
function hojas(valor: unknown, prefijo = ''): string[] {
	if (valor === null || typeof valor !== 'object') return [prefijo];

	return Object.entries(valor as Record<string, unknown>).flatMap(([clave, hijo]) =>
		hojas(hijo, prefijo ? `${prefijo}.${clave}` : clave)
	);
}

describe('archivos de idioma', () => {
	for (const idioma of IDIOMAS) {
		it(`${idioma}.yml es YAML válido`, () => {
			// El test más importante es este: `cargar` tira si el archivo no
			// parsea, y un valor con `": "` sin comillas no parsea. Que el
			// archivo cargue es justamente lo que se rompió las dos veces.
			const documento = cargar(idioma) as Record<string, unknown>;

			expect(documento).toBeObject();
			expect(hojas(documento).length).toBeGreaterThan(100);

			// `_version` es un número; el resto son frases. Nada de valores
			// nulos: una clave sin texto se muestra vacía en la interfaz.
			for (const ruta of hojas(documento)) {
				const valor = ruta.split('.').reduce<any>((nodo, parte) => nodo?.[parte], documento);

				expect(['string', 'number'], `${idioma}.yml → ${ruta}`).toContain(typeof valor);
			}
		});
	}

	it('los dos idiomas tienen exactamente las mismas claves', () => {
		const [es, en] = IDIOMAS.map((idioma) => new Set(hojas(cargar(idioma))));

		const faltanEnIngles = [...es].filter((clave) => !en.has(clave));
		const faltanEnEspanol = [...en].filter((clave) => !es.has(clave));

		expect(faltanEnIngles).toEqual([]);
		expect(faltanEnEspanol).toEqual([]);
	});

	it('los textos con parámetros usan {0} en los dos idiomas', () => {
		// El t() propio no interpola por nombre: reemplaza {0}, {1}… Si un
		// idioma trae el marcador y el otro no, en ese idioma el dato no
		// aparece nunca.
		const documentos = IDIOMAS.map((idioma) => cargar(idioma) as Record<string, unknown>);
		const claves = hojas(documentos[0]);

		for (const clave of claves) {
			const valores = documentos.map((documento) =>
				clave.split('.').reduce<any>((nodo, parte) => nodo?.[parte], documento)
			);

			if (valores.some((valor) => typeof valor !== 'string')) continue;

			const marcadores = valores.map((valor: string) =>
				(valor.match(/\{\d+\}/g) ?? []).sort().join(',')
			);

			expect(marcadores[1], `${clave} en en.yml`).toBe(marcadores[0]);
		}
	});
});
