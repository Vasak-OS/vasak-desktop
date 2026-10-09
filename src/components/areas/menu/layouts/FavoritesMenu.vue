<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */
/** biome-ignore-all lint/correctness/noUnusedVariables: usados en la plantilla */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed } from 'vue';
import FavoritesArea from '@/components/areas/menu/FavoritesArea.vue';
import PlacesColumn from '@/components/areas/menu/PlacesColumn.vue';
import type { MenuController } from '@/tools/composables/useMenuController';
import type { MenuConfig } from '@/tools/menu-config';
import { allApps, resolveFavorites } from '@/tools/menu-favorites';

/**
 * La variante de favoritos: una cuadrícula grande con las aplicaciones fijadas y,
 * al costado, los lugares (vasak-desktop#203). Pensada para quien entra al menú a
 * abrir lo de siempre; para todo lo demás está el buscador del marco.
 *
 * A lo ancho van lado a lado; en angosto los favoritos quedan arriba y los
 * lugares abajo —una columna por vez—. Fijar y desfijar se hace desde el clic
 * derecho, acá y en la lista de las otras variantes.
 */
const props = defineProps<{ controller: MenuController; menu: MenuConfig }>();

const { menuData } = props.controller;

const { t } = useI18n();

const favoriteApps = computed(() =>
	resolveFavorites(allApps(menuData.value), props.menu.favorites)
);
</script>

<template>
  <div class="@container flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto @2xl:flex-row @2xl:overflow-visible">
    <div class="min-h-0 flex-1 @2xl:overflow-y-auto">
      <FavoritesArea :apps="favoriteApps" />
    </div>

    <nav class="shrink-0 @2xl:w-56 @2xl:min-h-0 @2xl:overflow-y-auto" :aria-label="t('views.menu.places.title')">
      <PlacesColumn />
    </nav>
  </div>
</template>
