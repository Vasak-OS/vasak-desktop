<script setup lang="ts">
/**
 * Los accesos fijos del panel, en su píldora (vasak-desktop#151, configurable
 * desde #251).
 *
 * La lista sale de la configuración (`panel.pinned`): ausente, los dos de
 * siempre —Configuración y Archivos—; presente, lo que diga. La resolución vive
 * en `pinned-apps.ts`.
 *
 * **Cómo se lanzan:** por `open_app` del backend, no por el plugin de shell.
 * `Command.create` sólo puede lanzar lo que la capacidad `shell:allow-spawn`
 * nombra una por una, así que servía para los dos predeterminados pero no para
 * una aplicación anclada cualquiera. `open_app` recibe la ruta del `.desktop`
 * —el `id` de la entrada—, lee su `Exec=` y lo lanza; es el mismo camino que usa
 * el menú de inicio, y no necesita que cada aplicación esté en la capacidad.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { PanelPill, TrayIconButton } from '@vasakgroup/vue-libvasak';
import { openApp } from '@/services/app.service';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { type PinnedApp, pinnedLabel } from '@/tools/pinned-apps';
import { logError } from '@/utils/logger';

const { t } = useI18n();
const { vertical, hasSurface, pinnedApps } = usePanelConfig();

/** El nombre visible: traducido en las predeterminadas, literal en las de config. */
const label = (app: PinnedApp): string => pinnedLabel(app, t);

async function launch(app: PinnedApp): Promise<void> {
	try {
		await openApp({ path: app.id });
	} catch (error) {
		logError(`[panel] no se pudo abrir ${app.id}:`, error);
	}
}
</script>

<template>
  <PanelPill
    :interactive="false"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    :flat="hasSurface"
    flush
    role="group"
    :accessible-label="t('views.panel.pinnedAlt')"
    class="shrink-0"
    :class="vertical ? 'py-1' : 'px-1'"
    data-pinned-apps
  >
    <TrayIconButton
      v-for="app in pinnedApps"
      :key="app.id"
      :name="app.icon"
      :alt="label(app)"
      :tooltip="label(app)"
      custom-class="rounded-corner-full"
      :data-pinned-app="app.id"
      @click="launch(app)"
    />
  </PanelPill>
</template>
