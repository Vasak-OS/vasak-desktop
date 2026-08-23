<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import AppletFrame from '@/components/layouts/AppletFrame.vue';
import { toggleControlCenter } from '@/services/window.service';
import { useIcons } from '@/tools/composables/useReactiveIcon';
import { logError } from '@/utils/logger';

/**
 * El menú del clic derecho del panel.
 *
 * Sólo cosas del panel: lo que se hace con el escritorio —los widgets, el
 * fondo— tiene su propio clic derecho ahí, y ofrecerlo también acá dejaría dos
 * caminos para lo mismo, uno de ellos en el lugar equivocado.
 *
 * Es una ventana aparte porque el panel mide unos treinta píxeles de alto y un
 * menú dibujado adentro quedaría recortado. Ver `windows_apps/panel_menu.rs`.
 */
const { t } = useI18n();

const currentWindow = getCurrentWindow();

const { panelIcon, settingsIcon, notificationsIcon } = useIcons({
	panelIcon: 'preferences-system-windows',
	settingsIcon: 'preferences-system',
	notificationsIcon: 'preferences-desktop-notification',
});

const cerrar = () => {
	try {
		currentWindow.close();
	} catch {
		/* ya estaba cerrada */
	}
};

/** Cada opción cierra el menú, haya funcionado o no: dejarlo abierto después de
 * un error obligaría a cerrarlo a mano para poder ver lo que pasó. */
const ejecutar = async (accion: () => Promise<unknown>) => {
	try {
		await accion();
	} catch (error) {
		logError('[menú del panel] No se pudo ejecutar la opción:', error);
	} finally {
		cerrar();
	}
};

const opciones = [
	{
		id: 'panel',
		icon: panelIcon,
		label: () => t('views.applets.panelMenu.panelSettings'),
		run: () => invoke('open_settings_section', { section: 'appearance-panel' }),
	},
	{
		id: 'notifications',
		icon: notificationsIcon,
		label: () => t('views.applets.panelMenu.notifications'),
		run: () => toggleControlCenter(),
	},
	{
		id: 'system',
		icon: settingsIcon,
		label: () => t('views.applets.panelMenu.systemSettings'),
		run: () => invoke('open_settings'),
	},
];
</script>

<template>
  <AppletFrame :close-fn="cerrar">
    <nav class="flex h-full flex-col gap-1" :aria-label="t('views.applets.panelMenu.title')">
      <button
        v-for="opcion in opciones"
        :key="opcion.id"
        type="button"
        class="flex items-center gap-3 rounded-corner px-3 py-2 text-left text-sm text-tx-main transition-colors hover:bg-primary hover:text-tx-on-primary"
        @click="ejecutar(opcion.run)"
      >
        <img :src="opcion.icon.value" alt="" class="h-5 w-5 shrink-0" />
        <span class="truncate">{{ opcion.label() }}</span>
      </button>
    </nav>
  </AppletFrame>
</template>
