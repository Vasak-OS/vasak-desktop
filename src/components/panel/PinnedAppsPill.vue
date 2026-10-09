<script setup lang="ts">
/**
 * Los accesos fijos del panel: Configuración y Archivos, en su píldora.
 *
 * Es la zona donde después van a ir las aplicaciones ancladas
 * (vasak-desktop#151, decisión del usuario del 03/10/2026): por eso es una
 * lista y no dos botones sueltos. Sumar una aplicación es sumar una entrada
 * a `PINNED_APPS`; anclar desde la interfaz todavía no existe.
 */
import { Command } from '@tauri-apps/plugin-shell';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { PanelPill, TrayIconButton } from '@vasakgroup/vue-libvasak';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { PINNED_APPS, type PinnedApp } from '@/tools/pinned-apps';
import { logError } from '@/utils/logger';

const { t } = useI18n();
const { vertical, hasSurface } = usePanelConfig();

async function launch(app: PinnedApp): Promise<void> {
	try {
		await Command.create(app.command, []).spawn();
	} catch (error) {
		logError(`[panel] no se pudo abrir ${app.command}:`, error);
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
      v-for="app in PINNED_APPS"
      :key="app.id"
      :name="app.icon"
      :alt="t(app.label)"
      :tooltip="t(app.label)"
      custom-class="rounded-corner-full"
      :data-pinned-app="app.id"
      @click="launch(app)"
    />
  </PanelPill>
</template>
