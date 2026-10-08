<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */
/** biome-ignore-all lint/correctness/noUnusedVariables: usados en la plantilla */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ListCard, ListGroup, ListRow } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import FavoritesArea from '@/components/areas/menu/FavoritesArea.vue';
import MenuArea from '@/components/areas/menu/MenuArea.vue';
import PlacesColumn from '@/components/areas/menu/PlacesColumn.vue';
import type { MenuController } from '@/tools/composables/useMenuController';
import type { MenuConfig } from '@/tools/menu-config';
import { allApps, resolveFavorites } from '@/tools/menu-favorites';

/**
 * La variante clásica: una barra de categorías (y lugares) a la izquierda y la
 * lista de aplicaciones de la categoría a la derecha, al estilo de los menús de
 * toda la vida (vasak-desktop#203).
 *
 * A lo ancho van lado a lado; en angosto la columna de categorías pasa a ser una
 * tira horizontal arriba de la lista —una columna por vez, sin cortar nada—. Las
 * aplicaciones son las mismas `AppMenuCard` de la lista compacta, así que fijar
 * desde el clic derecho sigue andando.
 */
const props = defineProps<{ controller: MenuController; menu: MenuConfig }>();

const { categorySelected, appsOfCategory, categoryEntries, menuData } = props.controller;

const { t } = useI18n();

const categories = computed(() => {
	const { all, others } = categoryEntries.value;
	return all ? [all, ...others] : others;
});

const favoriteApps = computed(() =>
	resolveFavorites(allApps(menuData.value), props.menu.favorites)
);
const showFavorites = computed(() => props.menu.showFavorites && favoriteApps.value.length > 0);

const select = (key: string) => {
	categorySelected.value = key;
};
</script>

<template>
  <div class="@container flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto @2xl:flex-row @2xl:overflow-visible">
    <!-- Categorías y lugares: columna a lo ancho, tira horizontal en angosto. -->
    <nav class="shrink-0 @2xl:w-56 @2xl:min-h-0 @2xl:overflow-y-auto" :aria-label="t('views.menu.categories')">
      <ListGroup role="listbox" :label="t('views.menu.categories')" :divided="false">
        <ListRow
          v-for="([key, value]) in categories"
          :key="key"
          role="option"
          :icon="value.icon"
          :title="t(value.description)"
          :selected="key === categorySelected"
          @click="select(key)"
        />
      </ListGroup>

      <PlacesColumn v-if="menu.showPlaces" class="mt-4" />
    </nav>

    <!-- Las aplicaciones de la categoría, con los favoritos arriba si se piden. -->
    <div class="flex min-h-0 flex-1 flex-col gap-4">
      <FavoritesArea v-if="showFavorites" :apps="favoriteApps" class="shrink-0" />
      <ListCard custom-class="min-h-0 flex-1">
        <div class="h-full min-w-0 flex-1 overflow-y-auto">
          <MenuArea :apps="appsOfCategory" />
        </div>
      </ListCard>
    </div>
  </div>
</template>
