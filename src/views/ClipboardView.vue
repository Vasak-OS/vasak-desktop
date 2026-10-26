<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */
/** biome-ignore-all lint/correctness/noUnusedVariables: usados en la plantilla */
/**
 * El historial del portapapeles, estilo ChromeOS (Buscar+V) — vasak-desktop#217.
 *
 * Es una superposición de capa, de la familia del centro de control: el backend
 * la construye escondida al iniciar y la muestra con el atajo. Escape y perder el
 * foco la cierran del lado del backend (`dismiss_on_unfocus`), así que la página
 * sólo pide la lista al abrirse y la vuelve a pedir en cada apertura (la ventana
 * se esconde, nunca se destruye, así que Vue no se vuelve a montar).
 *
 * Al elegir una entrada se pone en el portapapeles y se cierra. Sólo texto en
 * v1; las imágenes quedan para después.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { EmptyState, ListRow, SearchField, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, ref } from 'vue';
import {
	clipboardClear,
	clipboardHistory,
	clipboardPick,
	hideClipboard,
} from '@/services/clipboard.service';
import { type ClipboardEntry, filterEntries } from '@/tools/clipboard-filter';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const entries = ref<ClipboardEntry[]>([]);
const filter = ref('');

const isEmpty = computed(() => entries.value.length === 0);
const shown = computed(() => filterEntries(entries.value, filter.value));

async function refresh(): Promise<void> {
	try {
		entries.value = await clipboardHistory();
	} catch (error) {
		logError('[clipboard] no se pudo leer el historial', error);
	}
}

async function pick(id: number): Promise<void> {
	try {
		await clipboardPick(id);
	} catch (error) {
		logError('[clipboard] no se pudo elegir la entrada', error);
	}
	filter.value = '';
	await hideClipboard();
}

async function clearAll(): Promise<void> {
	try {
		await clipboardClear();
		entries.value = [];
	} catch (error) {
		logError('[clipboard] no se pudo limpiar el historial', error);
	}
}

// Primer montaje (se abre por primera vez) y cada apertura posterior.
refresh();
useSharedEvent('window-shown', () => {
	filter.value = '';
	refresh();
});
</script>

<template>
  <main class="bg-ui-shell h-screen w-screen rounded-corner-m window-border flex flex-col gap-2 p-2 overflow-hidden">
    <div class="flex shrink-0 items-center gap-2">
      <SearchField
        v-model="filter"
        :label="t('views.clipboard.search')"
        :disabled="isEmpty"
        autofocus
        class="min-w-0 grow" />
      <button
        v-if="!isEmpty"
        type="button"
        class="shrink-0 rounded-corner-l border border-ui-line p-2"
        :title="t('views.clipboard.clear')"
        :aria-label="t('views.clipboard.clear')"
        data-clear
        @click="clearAll"
      >
        <ThemeIcon name="edit-delete" type="symbol" :size="16" alt="" />
      </button>
    </div>

    <div v-if="isEmpty" class="flex flex-1 min-h-0 items-center justify-center">
      <EmptyState :title="t('views.clipboard.empty')" icon="edit-paste" />
    </div>
    <div v-else class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-1" data-list>
      <ListRow
        v-for="entry in shown"
        :key="entry.id"
        role="button"
        icon="edit-paste"
        icon-type="symbol"
        :title="entry.text"
        truncate
        @click="pick(entry.id)"
      />
    </div>
  </main>
</template>
