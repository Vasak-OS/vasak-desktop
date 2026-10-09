/**
 * Los `@/…` del mosaico «Mantener despierto» para montarlo en las pruebas
 * (vasak-desktop#179). El componible es el de verdad: lo que se dobla es el
 * `invoke` de Tauri y los eventos, desde la prueba.
 */
export { useKeepAwake } from '../../src/tools/composables/useKeepAwake';
