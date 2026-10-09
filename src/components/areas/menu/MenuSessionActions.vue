<script setup lang="ts">
import { ActionButton } from '@vasakgroup/vue-libvasak';

/**
 * Los botones de sesión del menú: Configuración, Apagar, Reiniciar, Cerrar
 * sesión y Suspender (vasak-desktop#203).
 *
 * Se sacó de `MenuView.vue` para que lo compartan todas las variantes y para que
 * la opción `menu.showSessionActions` sólo tenga que decidir si se muestra, sin
 * repetir la fila en cada layout.
 *
 * Son los botones de la librería, `ghost` y del tamaño de siempre (40): sin
 * escala ni giro al pasar, el velo neutro de Once UI y el icono simbólico del
 * tema, que sigue el color del texto.
 */
defineProps<{
	actions: Array<{ title: string; icon: string; handler: () => void }>;
}>();
</script>

<template>
  <div class="flex flex-wrap items-center gap-2">
    <ActionButton
      v-for="action in actions"
      :key="action.icon"
      label=""
      :icon="action.icon"
      :icon-alt="action.title"
      :title="action.title"
      variant="ghost"
      size="lg"
      @click="action.handler"
    />
  </div>
</template>
