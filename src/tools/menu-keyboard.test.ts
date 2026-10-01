import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { resolveMenuKey } from '@/tools/menu-keyboard';

const key = (name: string, defaultPrevented = false) => ({ key: name, defaultPrevented });

describe('el teclado sobre los resultados del menú', () => {
	test('Enter lanza el marcado', () => {
		expect(resolveMenuKey(key('Enter'), 3, 1)).toEqual({ kind: 'launch', index: 1 });
	});

	test('un Enter que ya atendió el resultado enfocado no lanza otra aplicación', () => {
		// El ítem de menú de la librería atiende Enter y cancela la acción por
		// omisión; el mismo evento sigue subiendo hasta el documento.
		expect(resolveMenuKey(key('Enter', true), 3, 1)).toEqual({ kind: 'none' });
		expect(resolveMenuKey(key('ArrowDown', true), 3, 1)).toEqual({ kind: 'none' });
	});

	test('las flechas recorren y dan la vuelta', () => {
		expect(resolveMenuKey(key('ArrowDown'), 3, 2)).toEqual({ kind: 'move', index: 0 });
		expect(resolveMenuKey(key('ArrowRight'), 3, 0)).toEqual({ kind: 'move', index: 1 });
		expect(resolveMenuKey(key('ArrowUp'), 3, 0)).toEqual({ kind: 'move', index: 2 });
		expect(resolveMenuKey(key('ArrowLeft'), 3, 2)).toEqual({ kind: 'move', index: 1 });
	});

	test('sin resultados, o con otra tecla, nada', () => {
		expect(resolveMenuKey(key('Enter'), 0, 0)).toEqual({ kind: 'none' });
		expect(resolveMenuKey(key('a'), 3, 0)).toEqual({ kind: 'none' });
	});

	test('el menú pasa sus teclas por acá', () => {
		const view = readFileSync(join(import.meta.dir, '..', 'views', 'MenuView.vue'), 'utf8');
		const handler = view.slice(view.indexOf('const onKeydown'), view.indexOf('</script>'));

		expect(handler).toContain('resolveMenuKey(event, list.length, selectedIndex.value)');
		// Y no decide por su cuenta con `event.key`, que es lo que lanzaba dos.
		expect(handler).not.toMatch(/event\.key ===/);
	});
});
