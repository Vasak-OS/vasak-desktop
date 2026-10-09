<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */

import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { DropdownMenuItem, EmptyState, ThemeIcon } from '@vasakgroup/vue-libvasak';
import MarqueeText from '@/components/MarqueeText.vue';
import { useAppLauncher } from '@/tools/composables/useAppLauncher';
import type { MenuApp } from '@/tools/menu-favorites';

/**
 * La grilla de aplicaciones favoritas del menú (vasak-desktop#203).
 *
 * Cada favorito es el ítem de menú de la librería con el icono arriba y el
 * nombre abajo: Once UI en la forma, el icono del tema. Tocarlo lanza la
 * aplicación; el clic derecho la desfija. Lanzar y fijar/desfijar salen de
 * `useAppLauncher`, compartido con la fila y el mosaico.
 *
 * Recibe las aplicaciones ya resueltas (`resolveFavorites`): una que se
 * desinstaló deja de aparecer sin que haya que limpiar la lista guardada.
 */
const props = defineProps<{ apps: MenuApp[] }>();

const { t } = useI18n();
const { launch, toggleFavoriteFor } = useAppLauncher();

/**
 * El clic derecho se atiende por delegación en el contenedor (un `div` nativo):
 * así no hay que engancharlo en cada ítem de la librería, que no declara el
 * evento. Busca el favorito por su `data-fav-path` y abre el menú de fijar.
 */
const onContextMenu = (event: MouseEvent) => {
	const target = (event.target as HTMLElement | null)?.closest('[data-fav-path]');
	const path = target?.getAttribute('data-fav-path');
	if (!path) return;
	const app = props.apps.find((candidate) => candidate.path === path);
	if (app) void toggleFavoriteFor(app, event);
};
</script>

<template>
  <EmptyState
    v-if="props.apps.length === 0"
    :title="t('views.menu.favorites.empty')"
    icon="emblem-favorite"
  />
  <div
    v-else
    role="menu"
    :aria-label="t('views.menu.favorites.title')"
    class="grid grid-cols-[repeat(auto-fill,minmax(5rem,1fr))] gap-2"
    @contextmenu="onContextMenu"
  >
    <DropdownMenuItem
      v-for="app in props.apps"
      :key="app.path"
      :title="app.name"
      :data-fav-path="app.path"
      class="min-w-0 flex-col items-center gap-1 text-center"
      @select="launch(app)"
    >
      <ThemeIcon :name="app.icon" :size="48" alt="" />
      <MarqueeText :text="app.name" class="text-label-xs text-tx-main" />
    </DropdownMenuItem>
  </div>
</template>
