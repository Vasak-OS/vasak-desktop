<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: imports used in template */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { RevealTransition } from '@vasakgroup/vue-libvasak';
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
	// La categoría elegida: al cambiar, la lista entera cruza en una sola
	// transición (§5.5), no app por app. Sin ella no hay animación.
	sectionKey: {
		type: [String, Number],
		default: undefined,
	},
});

const { t } = useI18n();
</script>

<template>
  <!-- El mismo RevealTransition out-in que el menú usa para la búsqueda: una
       sola animación al cambiar de categoría, no una por aplicación. -->
  <RevealTransition mode="out-in">
    <div :key="sectionKey" role="menu" :aria-label="t('views.menu.applications')" class="flex min-w-0 flex-col gap-0.5">
      <AppMenuCard v-for="app in apps" :key="app.name" :app="app" />
    </div>
  </RevealTransition>
</template>
