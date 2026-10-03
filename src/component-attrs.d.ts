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
 *
 * Los `data-*` también, sobre un componente (vasak-desktop#151): marcan su
 * raíz para encontrarla después —las píldoras del panel con `data-tray-entry`,
 * que la bandeja usa para saber si tiene algo adentro—, y Vue los deja caer en
 * la raíz igual que `title`.
 */
declare module 'vue' {
	interface AllowedComponentProps {
		title?: string;
		[attribute: `aria-${string}`]: unknown;
		[attribute: `data-${string}`]: unknown;
		// `vue-tsc` comprueba los atributos de un componente ya pasados a
		// camelCase: `data-tray-entry` llega como `dataTrayEntry`.
		[attribute: `data${Capitalize<string>}`]: unknown;
	}
}

export {};
