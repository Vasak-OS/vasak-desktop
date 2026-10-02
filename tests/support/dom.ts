/**
 * El DOM de las pruebas que montan componentes (happy-dom).
 *
 * Va como `preload` de `bun test` (ver `bunfig.toml`) y no dentro de cada
 * archivo: `@vue/runtime-dom` lee `document` **una vez**, al importarse, y
 * `bun test` comparte la caché de módulos entre archivos. Si el primero en
 * importar `vue` lo hace sin DOM —`menu-app-row.test.ts`, que dibuja en el
 * servidor—, montar después falla con «null is not an object (evaluating
 * 'doc.createElement')» por más que el DOM se registre a tiempo en el archivo
 * que monta. Es lo mismo que hacen la librería y las aplicaciones.
 *
 * Sólo el DOM: los dobles de Tauri los pone cada archivo que los necesita
 * (`mount-sfc.ts`), para no cambiar el ambiente de las demás pruebas.
 */
import { GlobalRegistrator } from '@happy-dom/global-registrator';

GlobalRegistrator.register();
