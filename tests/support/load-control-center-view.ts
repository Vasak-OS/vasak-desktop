/**
 * Compila `ControlCenterView.vue` con cada `@/…` mandado a los dobles.
 *
 * Cada componente del escritorio es el doble de su nombre, o un doble mudo si
 * la prueba no lo conoce: la vista suma controles seguido (la luz nocturna, el
 * micrófono…) y las pruebas no tienen por qué romperse con cada uno. Lo que se
 * importa por nombre sale del mismo módulo de dobles.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { compileScript, parse } from 'vue/compiler-sfc';
import { ROOT } from './mount-sfc';

export async function loadControlCenterView(workdir: string, doublesPath: string) {
	const file = join(ROOT, 'src/views/ControlCenterView.vue');
	const { descriptor } = parse(readFileSync(file, 'utf8'), { filename: file });
	const compiled = compileScript(descriptor, { id: 'ControlCenterView', inlineTemplate: true });
	const code = `import * as __doubles from ${JSON.stringify(doublesPath)};\n${compiled.content
		.replace(/import\s+type\s+\{[^}]*\}\s+from\s+(['"])@\/[^'"]+\1;?/g, '')
		.replace(
			/import\s+(\w+)\s+from\s+(['"])@\/[^'"]+\.vue\2;?/g,
			"const $1 = __doubles.$1 ?? __doubles.silent('$1');"
		)
		.replace(
			/import\s+\{([^}]*)\}\s+from\s+(['"])@\/[^'"]+\2;?/g,
			(_all, names: string) =>
				`const {${names.replace(/\btype\s+\w+\s*,?/g, '').replace(/\s+as\s+/g, ': ')}} = __doubles;`
		)}`;
	const out = join(workdir, 'ControlCenterView.ts');
	writeFileSync(out, code);
	return (await import(out)).default;
}
