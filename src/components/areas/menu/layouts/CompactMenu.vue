<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */
/** biome-ignore-all lint/correctness/noUnusedVariables: usados en la plantilla */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ListCard } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import FavoritesArea from '@/components/areas/menu/FavoritesArea.vue';
import MenuArea from '@/components/areas/menu/MenuArea.vue';
import PlacesColumn from '@/components/areas/menu/PlacesColumn.vue';
import CategoryMenuPill from '@/components/buttons/CategoryMenuPill.vue';
import WidgetSlot from '@/components/widgets/WidgetSlot.vue';
import type { MenuController } from '@/tools/composables/useMenuController';
import type { MenuConfig } from '@/tools/menu-config';
import { allApps, resolveFavorites } from '@/tools/menu-favorites';
import type { WidgetType } from '@/tools/widgets/catalog';

/**
 * La vista ociosa de la variante compacta: la distribución de tres zonas de
 * vasak-desktop#74 (vasak-desktop#203).
 *
 * Es **equivalente al menú de hoy** cuando todas las opciones están en su valor
 * de fábrica (clima en el hueco, sin lugares ni favoritos): la lista de
 * aplicaciones de la categoría a la izquierda, y las categorías con el widget a
 * la derecha. Las opciones sólo **agregan** piezas al hueco de la derecha; no
 * cambian ese formato por omisión. El marco (usuario, buscador, acciones, hero)
 * y los estados de búsqueda y vacío los pone `MenuView`.
 */
const props = defineProps<{ controller: MenuController; menu: MenuConfig }>();

const { categorySelected, appsOfCategory, categoryEntries, menuData } = props.controller;

const { t } = useI18n();

const favoriteApps = computed(() =>
	resolveFavorites(allApps(menuData.value), props.menu.favorites)
);
const showFavorites = computed(() => props.menu.showFavorites && favoriteApps.value.length > 0);
const showWidget = computed(() => props.menu.widget !== 'none');
/** El tipo del widget, ya sin `'none'` (sólo se usa cuando hay widget). */
const widgetType = computed(() => props.menu.widget as WidgetType);
const showPlaces = computed(() => props.menu.showPlaces);
/** Si hay algo que mostrar en el hueco de la derecha, debajo de las categorías. */
const hasSide = computed(() => showWidget.value || showPlaces.value || showFavorites.value);
/** El caso de siempre: sólo el widget, sin apilar nada. */
const onlyWidget = computed(() => showWidget.value && !showPlaces.value && !showFavorites.value);
</script>

<template>
  <div
    class="grid min-h-0 flex-1 grid-cols-1 content-start gap-4 overflow-y-auto @3xl:grid-cols-3 @3xl:content-normal @3xl:overflow-visible"
  >
    <!-- Las zonas son tarjetas de la librería (`ListCard`): la superficie de lo
         que se apoya, el canto fino y el radio `l`. -->
    <ListCard custom-class="h-72 min-h-0 @3xl:h-full">
      <div class="h-full min-w-0 flex-1 overflow-y-auto">
        <MenuArea :apps="appsOfCategory" />
      </div>
    </ListCard>

    <div
      class="grid min-h-0 gap-4 @3xl:col-span-2 @3xl:h-full @3xl:min-h-0"
      :class="hasSide ? 'grid-rows-[auto_18rem] @max-sm:grid-rows-[auto_30rem] @3xl:grid-rows-[1fr_2fr]' : ''"
    >
      <ListCard custom-class="@3xl:min-h-0">
        <div class="grid h-full min-h-0 min-w-0 flex-1 grid-cols-1 gap-3 @md:grid-cols-[1fr_2fr]">
          <div v-if="categoryEntries.all" role="menu" :aria-label="t('views.menu.categories')" class="flex min-h-14 items-center justify-center">
            <CategoryMenuPill
              :category="categoryEntries.all[0]"
              :image="categoryEntries.all[1].icon"
              :label="t(categoryEntries.all[1].description)"
              v-model:categorySelected="categorySelected"
              large
            />
          </div>

          <div role="menu" :aria-label="t('views.menu.categories')" class="grid min-h-0 grid-cols-[repeat(auto-fit,minmax(4rem,1fr))] gap-3 @md:grid-cols-3 @md:grid-rows-2">
            <CategoryMenuPill
              v-for="([key, value]) in categoryEntries.others.slice(0, 6)"
              :key="key"
              :category="key"
              :image="value.icon"
              :label="t(value.description)"
              v-model:categorySelected="categorySelected"
            />
          </div>
        </div>
      </ListCard>

      <!-- El hueco de la derecha: el widget elegible y, si se piden, los
           favoritos y los lugares. Por omisión es sólo el widget, igual que
           hoy. -->
      <div v-if="hasSide" class="min-h-0">
        <WidgetSlot v-if="onlyWidget" :type="widgetType" />
        <div v-else class="flex h-full min-h-0 flex-col gap-3 overflow-y-auto">
          <FavoritesArea v-if="showFavorites" :apps="favoriteApps" />
          <div v-if="showWidget" class="min-h-48 shrink-0">
            <WidgetSlot :type="widgetType" />
          </div>
          <PlacesColumn v-if="showPlaces" />
        </div>
      </div>
    </div>
  </div>
</template>
