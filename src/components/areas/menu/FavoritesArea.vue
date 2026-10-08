<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */

import { showContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { DropdownMenuItem, EmptyState, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { openApp as sysOpenApp } from '@/services/app.service';
import { dismissMenu } from '@/services/window.service';
import { useMenuConfig } from '@/tools/composables/useMenuConfig';
import type { MenuApp } from '@/tools/menu-favorites';
import { logError } from '@/utils/logger';

/**
 * La grilla de aplicaciones favoritas del menú (vasak-desktop#203).
 *
 * Cada favorito es el ítem de menú de la librería con el icono arriba y el
 * nombre abajo: Once UI en la forma, el icono del tema. Tocarlo lanza la
 * aplicación; el menú contextual la desfija. Fijar se hace desde la lista de
 * aplicaciones (`AppMenuCard`).
 *
 * Recibe las aplicaciones ya resueltas (`resolveFavorites`): una que se
 * desinstaló deja de aparecer sin que haya que limpiar la lista guardada.
 */
const props = defineProps<{ apps: MenuApp[] }>();

const { t } = useI18n();
const { toggleFavoritePath } = useMenuConfig();

const launch = async (app: MenuApp) => {
	try {
		await sysOpenApp({ path: app.path } as any);
	} catch (error) {
		logError('Error al abrir aplicación:', error);
	} finally {
		void dismissMenu();
	}
};

/**
 * El clic derecho se atiende por delegación en el contenedor (un `div` nativo):
 * así no hay que engancharlo en cada ítem de la librería, que no declara el
 * evento. Busca la ruta en el `data-fav-path` del favorito tocado.
 */
const onContextMenu = async (event: MouseEvent) => {
	const target = (event.target as HTMLElement | null)?.closest('[data-fav-path]');
	const path = target?.getAttribute('data-fav-path');
	if (!path) return;
	event.preventDefault();
	try {
		const chosen = await showContextMenu(
			[{ id: 'unpin', label: t('views.menu.favorites.unpin'), icon: 'edit-delete' }],
			event
		);
		if (chosen?.id === 'unpin') await toggleFavoritePath(path);
	} catch (error) {
		logError('No se pudo abrir el menú del favorito:', error);
	}
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
      class="flex-col gap-1 text-center"
      @select="launch(app)"
    >
      <ThemeIcon :name="app.icon" :size="40" alt="" />
      <span class="w-full truncate text-label-xs text-tx-main">{{ app.name }}</span>
    </DropdownMenuItem>
  </div>
</template>
