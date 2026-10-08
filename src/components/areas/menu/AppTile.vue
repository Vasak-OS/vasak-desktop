<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */
import { showContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { DropdownMenuItem, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { openApp as sysOpenApp } from '@/services/app.service';
import { dismissMenu } from '@/services/window.service';
import { useMenuConfig } from '@/tools/composables/useMenuConfig';
import { isFavorite, type MenuApp } from '@/tools/menu-favorites';
import { logError } from '@/utils/logger';

/**
 * Una aplicación como mosaico: el icono arriba y el nombre abajo (vasak-desktop
 * #203). Es la pieza de las variantes de grilla y de mosaicos; la fila con icono
 * al costado es `AppMenuCard`.
 *
 * Es el ítem de menú de la librería (`DropdownMenuItem`) en columna: Once UI en
 * la forma, el icono del tema. Tocarlo lanza la aplicación; el clic derecho la
 * fija o la desfija de los favoritos, igual que en la lista.
 */
const props = withDefaults(defineProps<{ app: MenuApp; size?: 'md' | 'lg' }>(), {
	size: 'md',
});

const { t } = useI18n();
const { menu, toggleFavoritePath } = useMenuConfig();

const launch = async () => {
	try {
		await sysOpenApp({ path: props.app.path } as any);
	} catch (error) {
		logError('Error al abrir aplicación:', error);
	} finally {
		void dismissMenu();
	}
};

const onContextMenu = async (event: MouseEvent) => {
	event.preventDefault();
	const pinned = isFavorite(menu.value.favorites, props.app.path);
	try {
		const chosen = await showContextMenu(
			[
				{
					id: 'toggle-favorite',
					label: pinned ? t('views.menu.favorites.unpin') : t('views.menu.favorites.pin'),
					icon: pinned ? 'edit-delete' : 'emblem-favorite',
				},
			],
			event
		);
		if (chosen?.id === 'toggle-favorite') await toggleFavoritePath(props.app.path);
	} catch (error) {
		logError('No se pudo abrir el menú de la aplicación:', error);
	}
};

/** El clic derecho se engancha a mano: la librería no declara el evento nativo. */
const item = ref<{ $el?: HTMLElement } | null>(null);
onMounted(() => item.value?.$el?.addEventListener('contextmenu', onContextMenu));
onBeforeUnmount(() => item.value?.$el?.removeEventListener('contextmenu', onContextMenu));
</script>

<template>
  <DropdownMenuItem
    ref="item"
    :title="app.name"
    class="h-full flex-col justify-center gap-1 text-center"
    @select="launch">
    <ThemeIcon :name="app.icon" :size="size === 'lg' ? 48 : 40" alt="" />
    <span class="w-full truncate text-label-xs text-tx-main">{{ app.name }}</span>
  </DropdownMenuItem>
</template>
