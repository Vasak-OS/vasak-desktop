<script setup lang="ts">
/**
 * Los espacios de trabajo del panel (vasak-desktop#151): «1 2 3 4 5 6» en una
 * píldora, con el actual en el primario.
 *
 * Cuántos hay lo dice Wayfire —los arma en una grilla, y una de 3 × 2 son
 * seis—, y el backend avisa con `workspaces-changed` al cambiar de espacio o
 * de pantalla. Tocar un número pasa a ése. Sin Wayfire, o con un solo espacio,
 * no hay nada que elegir y la píldora no se dibuja.
 *
 * Con un panel que tiene superficie propia (flotante, barra, dock) va `flat`,
 * como el resto de las píldoras: sin eso pintaba su fondo encima del panel.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { WorkspaceSwitcher } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { getWorkspaces, switchWorkspace, type WorkspaceState } from '@/services/compositor.service';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

const { t } = useI18n();
const { vertical, hasSurface } = usePanelConfig();

const state = ref<WorkspaceState | null>(null);
const visible = computed(() => (state.value?.count ?? 0) > 1);
const labels = computed(() =>
	Array.from({ length: state.value?.count ?? 0 }, (_, index) =>
		t('views.panel.workspace').replace('{0}', String(index + 1))
	)
);

async function select(index: number): Promise<void> {
	if (!state.value) return;
	// Se marca en el acto; el aviso de Wayfire lo confirma, o lo corrige si
	// el cambio no se pudo hacer.
	const previous = state.value;
	state.value = { ...previous, active: index };
	try {
		await switchWorkspace(index);
	} catch (error) {
		state.value = previous;
		logError('[panel] no se pudo cambiar de espacio de trabajo:', error);
	}
}

onMounted(async () => {
	try {
		state.value = await getWorkspaces();
	} catch (error) {
		logError('[panel] no se pudieron leer los espacios de trabajo:', error);
	}
});

useSharedEvent<WorkspaceState | null>('workspaces-changed', (payload) => {
	state.value = payload ?? null;
});
</script>

<template>
  <WorkspaceSwitcher
    v-if="visible && state"
    :count="state.count"
    :model-value="state.active"
    :label="t('views.panel.workspaces')"
    :labels="labels"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    :flat="hasSurface"
    @change="select"
  />
</template>
