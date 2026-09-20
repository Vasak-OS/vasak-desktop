import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
	CLASES_DE_LA_BARRA,
	esVertical,
	POSICIONES_DEL_PANEL,
	posicionDelPanel,
} from './posicion-del-panel';

describe('posicionDelPanel', () => {
	test('sin nada puesto, el panel va arriba', () => {
		// La sección `panel` existe desde antes que esta clave —lleva los
		// interruptores de los indicadores—, así que el caso normal de una
		// instalación vieja es que la sección esté y la clave no.
		expect(posicionDelPanel({})).toBe('top');
		expect(posicionDelPanel(null)).toBe('top');
		expect(posicionDelPanel({ panel: {} })).toBe('top');
		expect(posicionDelPanel({ panel: { weather: false } })).toBe('top');
	});

	test('los cuatro lados se leen', () => {
		for (const lado of POSICIONES_DEL_PANEL) {
			expect(posicionDelPanel({ panel: { position: lado } })).toBe(lado);
		}
	});

	test('cualquier otra cosa vale por arriba', () => {
		// El archivo se edita a mano. Con una aserción de tipo, un `"izquierda"`
		// llegaría hasta las clases de la barra y no coincidiría con ninguna: el
		// panel se dibujaría sin acomodo.
		expect(posicionDelPanel({ panel: { position: 'izquierda' } })).toBe('top');
		expect(posicionDelPanel({ panel: { position: 3 } })).toBe('top');
		expect(posicionDelPanel({ panel: 'left' })).toBe('top');
	});
});

describe('esVertical', () => {
	test('a los costados el panel es una columna', () => {
		expect(esVertical('top')).toBe(false);
		expect(esVertical('bottom')).toBe(false);
		expect(esVertical('left')).toBe(true);
		expect(esVertical('right')).toBe(true);
	});
});

describe('las clases de la barra', () => {
	test('cada lado se estira sobre su eje y conserva el grosor', () => {
		// Los 38 píxeles que reserva la superficie son los mismos de los cuatro
		// lados: 36 de la barra (`h-9` / `w-9`) más 2 de margen contra el borde
		// de la pantalla. Lo que cambia es sobre qué eje se estira.
		for (const lado of POSICIONES_DEL_PANEL) {
			const clases = CLASES_DE_LA_BARRA[lado].split(' ');

			if (esVertical(lado)) {
				expect(clases).toContain('flex-col');
				expect(clases).toContain('w-9');
				expect(clases).toContain('h-[calc(100vh-8px)]');
			} else {
				expect(clases).toContain('flex-row');
				expect(clases).toContain('h-9');
				expect(clases).toContain('w-[calc(100%-8px)]');
			}
		}
	});

	test('el margen va contra el borde de la pantalla, no contra las ventanas', () => {
		// Que es lo que hace el panel de arriba desde siempre: 2 píxeles arriba y
		// nada abajo, así que la barra queda pegada a lo que haya debajo.
		expect(CLASES_DE_LA_BARRA.top).toContain('mt-0.5');
		expect(CLASES_DE_LA_BARRA.bottom).toContain('mb-0.5');
		expect(CLASES_DE_LA_BARRA.left).toContain('ml-0.5');
		expect(CLASES_DE_LA_BARRA.right).toContain('mr-0.5');
	});
});

describe('la interfaz y el backend leen la misma clave', () => {
	/**
	 * El backend ancla la superficie y la interfaz dibuja lo de adentro, cada
	 * uno leyendo `panel.position` por su cuenta. Si uno de los dos cambia de
	 * clave o de valores, el panel se ancla de un lado y se dibuja para el otro,
	 * y nada falla: queda una columna de iconos acostada en una barra
	 * horizontal.
	 */
	const rust = readFileSync(
		join(import.meta.dir, '../../src-tauri/src/posicion_del_panel.rs'),
		'utf8'
	);

	test('la sección y la clave son las mismas', () => {
		expect(rust).toContain('get("panel")');
		expect(rust).toContain('get("position")');
	});

	test('y los cuatro valores también', () => {
		for (const lado of POSICIONES_DEL_PANEL) {
			expect(rust).toContain(`"${lado}" =>`);
		}
	});
});
