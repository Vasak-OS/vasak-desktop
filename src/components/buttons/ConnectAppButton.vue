<script setup lang="ts">
import { ThemeIcon } from '@vasakgroup/vue-libvasak';
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
  <div class="group flex items-center gap-3 rounded-corner p-2 hover:bg-primary/20">
    <button type="button" class="flex min-w-0 flex-1 items-center gap-3 text-left" @click="emit('open')">
      <img v-if="app.icon" :src="app.icon" alt="" class="h-8 w-8 shrink-0" />
      <ThemeIcon v-else-if="themeName" :name="themeName" :size="32" />
      <span
        v-else
        aria-hidden="true"
        class="grid h-8 w-8 shrink-0 place-items-center rounded-corner bg-ui-surface font-semibold text-primary"
      >
        {{ initial }}
      </span>
      <span class="truncate">{{ app.label }}</span>
    </button>
    <slot name="actions" />
  </div>
</template>
