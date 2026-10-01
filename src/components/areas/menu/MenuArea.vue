<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: imports used in template */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import AppMenuCard from '@/components/cards/AppMenuCard.vue';

/**
 * Las aplicaciones de la categoría elegida, en una columna.
 *
 * Es un `menu` porque lo de adentro son ítems de menú de la librería: así un
 * lector de pantalla anuncia la lista como lo que es.
 */
defineProps({
	apps: {
		type: Array<any>,
		required: true,
	},
});

const { t } = useI18n();
</script>

<template>
  <div role="menu" :aria-label="t('views.menu.applications')" class="flex min-w-0 flex-col gap-0.5">
    <AppMenuCard v-for="app in apps" :key="app.name" :app="app" />
  </div>
</template>
