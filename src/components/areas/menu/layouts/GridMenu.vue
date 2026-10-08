<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */
/** biome-ignore-all lint/correctness/noUnusedVariables: usados en la plantilla */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { EmptyState, PageDots } from '@vasakgroup/vue-libvasak';
import { computed, ref, watch } from 'vue';
import AppTile from '@/components/areas/menu/AppTile.vue';
import CategoryMenuPill from '@/components/buttons/CategoryMenuPill.vue';
import type { MenuController } from '@/tools/composables/useMenuController';
import type { MenuConfig } from '@/tools/menu-config';
import type { MenuApp } from '@/tools/menu-favorites';

/**
 * La variante en grilla: las categorías como píldoras centradas arriba y las
 * aplicaciones de la elegida como una cuadrícula de mosaicos, paginada
 * (vasak-desktop#203). El buscador lo pone el marco (`MenuView`).
 *
 * La cuadrícula se adapta al ancho con columnas automáticas; cuando una
 * categoría tiene más aplicaciones que una página, los puntos de abajo cambian
 * de página. En angosto entra menos por fila y hay más páginas, sin cortar nada.
 */
const props = defineProps<{ controller: MenuController; menu: MenuConfig }>();

const { categorySelected, appsOfCategory, categoryEntries } = props.controller;

const { t } = useI18n();

/** Cuántos mosaicos por página. Fijo, para no depender de medir el contenedor. */
const PAGE_SIZE = 35;

const page = ref(0);

const apps = computed<MenuApp[]>(() => appsOfCategory.value as MenuApp[]);
const pageCount = computed(() => Math.max(1, Math.ceil(apps.value.length / PAGE_SIZE)));
const pageApps = computed(() =>
	apps.value.slice(page.value * PAGE_SIZE, page.value * PAGE_SIZE + PAGE_SIZE)
);

const categories = computed(() => {
	const { all, others } = categoryEntries.value;
	return all ? [all, ...others] : others;
});

// Al cambiar de categoría se vuelve a la primera página; y si la actual ya no
// existe (menos aplicaciones), se acota.
watch(categorySelected, () => {
	page.value = 0;
});
watch(pageCount, (count) => {
	if (page.value >= count) page.value = count - 1;
});
</script>

<template>
  <div class="@container flex min-h-0 flex-1 flex-col gap-4">
    <!-- Las categorías como píldoras centradas, con la elegida marcada. -->
    <div role="menu" :aria-label="t('views.menu.categories')" class="flex shrink-0 flex-wrap items-center justify-center gap-2">
      <CategoryMenuPill
        v-for="([key, value]) in categories"
        :key="key"
        :category="key"
        :image="value.icon"
        :label="t(value.description)"
        v-model:categorySelected="categorySelected"
      />
    </div>

    <EmptyState
      v-if="apps.length === 0"
      class="flex-1"
      :title="t('views.menu.noApps')"
      icon="application-x-executable"
    />
    <template v-else>
      <div
        role="menu"
        :aria-label="t('views.menu.applications')"
        class="grid min-h-0 flex-1 content-start gap-3 overflow-y-auto grid-cols-[repeat(auto-fill,minmax(5.5rem,1fr))]"
      >
        <AppTile v-for="app in pageApps" :key="app.path" :app="app" />
      </div>

      <PageDots
        v-if="pageCount > 1"
        class="shrink-0 justify-center"
        :count="pageCount"
        v-model="page"
        :label="t('views.menu.applications')"
      />
    </template>
  </div>
</template>
