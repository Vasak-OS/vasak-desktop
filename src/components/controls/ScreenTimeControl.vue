<script setup lang="ts">
/**
 * El botón del centro de control que abre el tablero de tiempo de pantalla
 * (vasak-desktop#150).
 *
 * La misma baldosa de la librería que la búsqueda de al lado, sin `pressed`:
 * abrir el tablero no prende ni apaga nada. El tablero es un applet anclado
 * (`screen-time` en `APPLETS`); sin botón del panel del que colgar, el
 * backend lo centra en el eje del panel. Antes de abrirlo se cierra el centro
 * de control: el tablero ocupa el centro de la pantalla, y dos superficies
 * flotando a la vez se pisan.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ToggleControl } from '@vasakgroup/vue-libvasak';
import { hideControlCenter, toggleApplet } from '@/services/window.service';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const openScreenTime = async () => {
	try {
		await hideControlCenter();
		await toggleApplet('screen-time');
	} catch (error) {
		logError('[ScreenTimeControl] no se pudo abrir el tablero:', error);
	}
};
</script>

<template>
  <ToggleControl
    name="preferences-system-time"
    type="symbol"
    :label="t('components.ScreenTimeControl.open')"
    @click="openScreenTime"
  />
</template>
