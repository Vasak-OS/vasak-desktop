<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: usados en la plantilla */
import { DropdownMenuItem, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { useAppLauncher } from '@/tools/composables/useAppLauncher';
import type { MenuApp } from '@/tools/menu-favorites';

/**
 * Una aplicación como mosaico: el icono arriba y el nombre abajo (vasak-desktop
 * #203). Es la pieza de las variantes de grilla y de mosaicos; la fila con icono
 * al costado es `AppMenuCard`.
 *
 * Es el ítem de menú de la librería (`DropdownMenuItem`) en columna: Once UI en
 * la forma, el icono del tema. Lanzar y fijar/desfijar con el clic derecho salen
 * de `useAppLauncher`, igual que en la lista y en la grilla de favoritos.
 */
const props = withDefaults(defineProps<{ app: MenuApp; size?: 'md' | 'lg' }>(), {
	size: 'md',
});

const { launch, toggleFavoriteFor } = useAppLauncher();

/** El clic derecho se engancha a mano: la librería no declara el evento nativo. */
const item = ref<{ $el?: HTMLElement } | null>(null);
const onContextMenu = (event: Event) => {
	void toggleFavoriteFor(props.app, event as MouseEvent);
};
onMounted(() => item.value?.$el?.addEventListener('contextmenu', onContextMenu));
onBeforeUnmount(() => item.value?.$el?.removeEventListener('contextmenu', onContextMenu));
</script>

<template>
  <DropdownMenuItem
    ref="item"
    :title="app.name"
    class="h-full flex-col justify-center gap-1 text-center"
    @select="launch(app)">
    <ThemeIcon :name="app.icon" :size="size === 'lg' ? 48 : 40" alt="" />
    <span class="w-full truncate text-label-xs text-tx-main">{{ app.name }}</span>
  </DropdownMenuItem>
</template>
