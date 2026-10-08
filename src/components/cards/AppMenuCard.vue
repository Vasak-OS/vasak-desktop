<script lang="ts" setup>
import { DropdownMenuItem, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { useAppLauncher } from '@/tools/composables/useAppLauncher';
import type { MenuApp } from '@/tools/menu-favorites';

/**
 * Una aplicación en la lista de la categoría del menú.
 *
 * Es el ítem de menú de la librería (`DropdownMenuItem`): la lista del menú es
 * literalmente un desplegable largo, y así toma la forma de Once UI —radio `m`,
 * el velo neutro `ui-hover` al pasar en lugar del relleno del primario, el
 * anillo de foco por dentro— sin que el menú se dibuje la suya. Antes era un
 * botón propio que crecía, se corría y se pintaba de rosa al pasar por encima.
 *
 * El icono a la izquierda y el nombre al lado, en fila. El nombre se parte en
 * dos líneas en vez de cortarse: no hay otro lugar donde leerlo entero.
 *
 * Lanzar y fijar/desfijar con el clic derecho salen de `useAppLauncher`, que lo
 * comparte con el mosaico y la grilla de favoritos (vasak-desktop#203).
 */
const props = defineProps<{ app: MenuApp }>();

const { launch, toggleFavoriteFor } = useAppLauncher();

/**
 * El clic derecho se engancha a mano sobre la raíz del ítem: `DropdownMenuItem`
 * no declara el evento nativo y pasárselo por plantilla no pasa el tipado. Así
 * el clic derecho sobre toda la fila fija o desfija la aplicación.
 */
const item = ref<{ $el?: HTMLElement } | null>(null);
const onContextMenu = (event: Event) => {
	void toggleFavoriteFor(props.app, event as MouseEvent);
};

onMounted(() => {
	item.value?.$el?.addEventListener('contextmenu', onContextMenu);
});

onBeforeUnmount(() => {
	item.value?.$el?.removeEventListener('contextmenu', onContextMenu);
});
</script>

<template>
  <!-- El icono va en la ranura `prefix` y el nombre en la principal: el ítem
       de la librería los pone en fila, icono a la izquierda y nombre al lado,
       como era el menú antes de la migración. Puestos los dos en la ranura
       principal caían adentro de la columna del texto, y el nombre quedaba
       debajo del icono. -->
  <DropdownMenuItem
    ref="item"
    :title="app.description"
    class="w-full"
    @select="launch(app)">
    <template #prefix>
      <ThemeIcon :name="app.icon" :size="32" alt="" />
    </template>
    <span class="break-words text-left" data-app-name>{{ app.name }}</span>
  </DropdownMenuItem>
</template>
