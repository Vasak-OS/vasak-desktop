<script setup lang="ts">
import { ListRow, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import type { ConnectApp } from '@/interfaces/connect';
import { initialFor, themeIconFor } from '@/tools/androidIcon';

const props = defineProps<{
	app: ConnectApp;
	running: boolean;
}>();

const emit = defineEmits<{
	open: [];
	close: [];
}>();

/**
 * Tres fuentes, y el orden importa.
 *
 * La ruta que manda el servicio gana: es el icono de verdad de la aplicación y
 * no uno parecido. Si no hay, se prueba el nombre del tema, que dibuja
 * `ThemeIcon` —así sigue al tema y entra en el planificador de recarga—. Y si no
 * hay ninguno de los dos queda la inicial, que es el caso de toda aplicación sin
 * mapeo.
 */
const themeName = computed(() => themeIconFor(props.app.package) ?? '');

const initial = computed(() => initialFor(props.app.label, props.app.package));
</script>

<template>
  <!-- La fila de la librería (`ListRow`): abre la aplicación en el teléfono.
       Lo que va a la derecha (cerrarla) llega por la ranura `actions`, y el
       botón corta el clic para no abrirla a la vez. -->
  <ListRow role="button" :title="app.label" truncate @click="emit('open')">
    <template #leading>
      <img v-if="app.icon" :src="app.icon" alt="" class="size-8 shrink-0" />
      <ThemeIcon v-else-if="themeName" :name="themeName" :size="32" />
      <span
        v-else
        aria-hidden="true"
        class="grid size-8 shrink-0 place-items-center rounded-corner-m border border-ui-line bg-ui-surface/70 font-semibold text-tx-main"
      >
        {{ initial }}
      </span>
    </template>
    <template v-if="$slots.actions" #trailing>
      <slot name="actions" />
    </template>
  </ListRow>
</template>
