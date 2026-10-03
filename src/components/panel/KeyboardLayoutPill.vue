<script setup lang="ts">
/**
 * La distribución de teclado del panel (vasak-desktop#151): «US», «LA».
 *
 * La lee Wayfire, y el backend avisa con `keyboard-layout-changed` cuando
 * cambia —por el atajo o desde acá—. Con más de una configurada, tocarla pasa
 * a la siguiente; con una sola informa y no promete un clic.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { PanelPill } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import {
	getKeyboardLayout,
	type KeyboardLayout,
	nextKeyboardLayout,
} from '@/services/compositor.service';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

const { t } = useI18n();
const { vertical } = usePanelConfig();

const layout = ref<KeyboardLayout | null>(null);
const switchable = computed(() => (layout.value?.count ?? 0) > 1);
const description = computed(() =>
	layout.value ? t('views.panel.keyboardLayout').replace('{0}', layout.value.name) : ''
);
const accessibleLabel = computed(() =>
	switchable.value
		? t('views.panel.keyboardLayoutNext').replace('{0}', description.value)
		: description.value
);

async function next(): Promise<void> {
	try {
		layout.value = (await nextKeyboardLayout()) ?? layout.value;
	} catch (error) {
		logError('[panel] no se pudo cambiar la distribución de teclado:', error);
	}
}

onMounted(async () => {
	try {
		layout.value = await getKeyboardLayout();
	} catch (error) {
		logError('[panel] no se pudo leer la distribución de teclado:', error);
	}
});

useSharedEvent<KeyboardLayout | null>('keyboard-layout-changed', (payload) => {
	layout.value = payload ?? null;
});
</script>

<template>
  <PanelPill
    v-if="layout"
    :label="layout.short"
    :interactive="switchable"
    :title="description"
    :accessible-label="accessibleLabel"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    class="shrink-0"
    data-keyboard-layout
    @click="next"
  />
</template>
