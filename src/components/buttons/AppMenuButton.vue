<script lang="ts" setup>
import { DropdownMenuItem, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { openApp as sysOpenApp } from '@/services/app.service';
import { dismissMenu } from '@/services/window.service';
import { logError } from '@/utils/logger';

/**
 * Un resultado de la búsqueda del menú: el icono solo, con el nombre en el
 * globo y como nombre accesible.
 *
 * Es el ítem de menú de la librería, igual que la fila de la lista. El que
 * marcan las flechas desde el campo es **lo elegido**, y lo elegido va con el
 * velo del acento (`ui-selected-accent`, decisión 4 de vue-libvasak#74); antes
 * crecía un 10 % y se corría al pasar por encima.
 */
const props = defineProps({
	app: {
		type: Object,
		required: true,
	},
	selected: {
		type: Boolean,
		default: false,
	},
});

const openApp = async () => {
	try {
		await sysOpenApp({ path: props.app.path } as any);
	} catch (error) {
		logError('Error al abrir aplicación:', error);
	} finally {
		// Esconder, no cerrar: ver `dismissMenu`.
		void dismissMenu();
	}
};
</script>

<template>
  <DropdownMenuItem
    :title="app.name"
    :class="selected ? 'bg-ui-selected-accent font-semibold' : ''"
    :aria-current="selected || undefined"
    @select="openApp">
    <ThemeIcon :name="app.icon" :size="40" :alt="app.name" />
  </DropdownMenuItem>
</template>
