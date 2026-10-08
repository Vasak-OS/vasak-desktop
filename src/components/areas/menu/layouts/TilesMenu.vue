<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */
/** biome-ignore-all lint/correctness/noUnusedVariables: usados en la plantilla */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { EmptyState, SectionHeading } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import AppTile from '@/components/areas/menu/AppTile.vue';
import FavoritesArea from '@/components/areas/menu/FavoritesArea.vue';
import type { MenuController } from '@/tools/composables/useMenuController';
import type { MenuConfig } from '@/tools/menu-config';
import { allApps, resolveFavorites } from '@/tools/menu-favorites';

/**
 * La variante de mosaicos: todas las categorías a la vez, cada una con su título
 * y una cuadrícula de mosaicos grandes (vasak-desktop#203). Es la vista para
 * recorrer con la vista en lugar de elegir una categoría a la vez.
 *
 * Una sola columna que desplaza, con las cuadrículas adaptándose al ancho: en
 * angosto entran menos mosaicos por fila, sin cortar nada. Fijar y desfijar, con
 * el clic derecho sobre cada mosaico.
 */
const props = defineProps<{ controller: MenuController; menu: MenuConfig }>();

const { categoryEntries, menuData } = props.controller;

const { t } = useI18n();

/** Las categorías reales (sin «todas»), que son las que agrupan. */
const sections = computed(() =>
	categoryEntries.value.others.filter(([, value]) => (value.apps?.length ?? 0) > 0)
);

const favoriteApps = computed(() =>
	resolveFavorites(allApps(menuData.value), props.menu.favorites)
);
// Activado pero vacío se muestra igual: `FavoritesArea` dibuja su estado vacío
// con la guía para fijar aplicaciones (igual que en el compacto, CodeRabbit #204).
const showFavorites = computed(() => props.menu.showFavorites);

const isEmpty = computed(() => sections.value.length === 0);
</script>

<template>
  <div class="@container flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto">
    <EmptyState
      v-if="isEmpty"
      class="flex-1"
      :title="t('views.menu.noApps')"
      icon="application-x-executable"
    />
    <template v-else>
      <section v-if="showFavorites" :aria-label="t('views.menu.favorites.title')">
        <SectionHeading :title="t('views.menu.favorites.title')" icon="emblem-favorite" class="mb-3" />
        <FavoritesArea :apps="favoriteApps" />
      </section>

      <section v-for="([key, value]) in sections" :key="key" :aria-label="t(value.description)">
        <SectionHeading :title="t(value.description)" :icon="value.icon" :count="value.apps.length" class="mb-3" />
        <div
          role="menu"
          :aria-label="t(value.description)"
          class="grid gap-3 grid-cols-[repeat(auto-fill,minmax(6.5rem,1fr))]"
        >
          <AppTile v-for="app in value.apps" :key="app.path" :app="app" size="lg" />
        </div>
      </section>
    </template>
  </div>
</template>
