/**
 * Los `@/…` de los componentes de «No molestar» para montarlos en las pruebas
 * (vasak-desktop#177). El componible es el de verdad: lo que se dobla es el
 * `invoke` de Tauri y los eventos, desde la prueba.
 */
export { useDoNotDisturb } from '../../src/tools/composables/useDoNotDisturb';
