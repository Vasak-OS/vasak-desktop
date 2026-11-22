/**
 * Compila una vista con cada `@/…` mandado a un módulo de dobles.
 *
 * Como `load-control-center-view.ts`, pero para cualquier vista: lo que se
 * importa por defecto (`.vue`) es el doble de su nombre o uno mudo; lo que se
 * importa por nombre sale del módulo de dobles.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { compileScript, parse } from 'vue/compiler-sfc';
import { ROOT } from './mount-sfc';

export async function loadView(workdir: string, source: string, doublesPath: string, name: string) {
	const file = join(ROOT, source);
	const { descriptor } = parse(readFileSync(file, 'utf8'), { filename: file });
	const compiled = compileScript(descriptor, { id: name, inlineTemplate: true });
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
	const out = join(workdir, `${name}.ts`);
	writeFileSync(out, code);
	return (await import(out)).default;
}
