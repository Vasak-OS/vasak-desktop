/**
 * Los `@/…` del brillo por monitor y del perfil de energía para montar sus
 * componentes en las pruebas (vasak-desktop#189). Los componibles y los
 * ayudantes son los de verdad: lo que se dobla es el `invoke` de Tauri y los
 * eventos, desde la prueba.
 */
export { brightnessIcon, brightnessPercentageClass } from '../../src/tools/brightness-look';
export { useDisplayBrightness } from '../../src/tools/composables/useDisplayBrightness';
export { usePowerProfile } from '../../src/tools/composables/usePowerProfile';
export { ddcNotices, monitorKey, monitorName } from '../../src/tools/display-brightness';
export { DEFAULT_PROFILES, profileIcon, profileLabelKey } from '../../src/tools/power-profile';
