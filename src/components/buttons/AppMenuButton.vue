<script lang="ts" setup>
import { DropdownMenuItem, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { useAppLauncher } from '@/tools/composables/useAppLauncher';
import type { MenuApp } from '@/tools/menu-favorites';

/**
 * Un resultado de la búsqueda del menú: el icono solo, con el nombre en el
 * globo y como nombre accesible.
 *
 * Es el ítem de menú de la librería, igual que la fila de la lista. El que
 * marcan las flechas desde el campo es **lo elegido**, y lo elegido va con el
 * velo del acento (`ui-selected-accent`, decisión 4 de vue-libvasak#74); antes
 * crecía un 10 % y se corría al pasar por encima.
 *
 * Lanzar y fijar/desfijar con el clic derecho salen de `useAppLauncher`, igual
 * que en la lista de categorías: así fijar favoritos también funciona desde la
 * búsqueda (bug #206).
 */
const props = defineProps<{ app: MenuApp; selected?: boolean }>();

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
    :class="selected ? 'bg-ui-selected-accent font-semibold' : ''"
    :aria-current="selected || undefined"
    @select="launch(app)">
    <ThemeIcon :name="app.icon" :size="40" :alt="app.name" />
  </DropdownMenuItem>
</template>
