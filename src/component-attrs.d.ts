/**
 * Los atributos de HTML que un componente deja caer en su raíz.
 *
 * Con `vueCompilerOptions.strictTemplates`, `vue-tsc` rechaza todo atributo
 * que el componente no declare como propiedad. Para los de la librería eso
 * dejaba afuera dos que sí hacen falta y que Vue entrega solo a la raíz —el
 * botón, el ítem de menú—: el globo nativo (`title`) y el estado para quien usa
 * un lector de pantalla (`aria-pressed`, `aria-current`). Sin ellos, pasar a
 * los componentes de la librería obligaba a perder el globo o el estado.
 *
 * Se declara el patrón `aria-*` y no cada nombre, como los `data-*` de
 * `tipos-de-plantilla.d.ts`: lo que se quiere permitir es la forma.
 */
declare module 'vue' {
	interface AllowedComponentProps {
		title?: string;
		[attribute: `aria-${string}`]: unknown;
	}
}

export {};
