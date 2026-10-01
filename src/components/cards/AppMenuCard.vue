<script lang="ts" setup>
import { DropdownMenuItem, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { openApp as sysOpenApp } from '@/services/app.service';
import { dismissMenu } from '@/services/window.service';
import { logError } from '@/utils/logger';

/**
 * Una aplicación en la lista de la categoría del menú.
 *
 * Es el ítem de menú de la librería (`DropdownMenuItem`): la lista del menú es
 * literalmente un desplegable largo, y así toma la forma de Once UI —radio `m`,
 * el velo neutro `ui-hover` al pasar en lugar del relleno del primario, el
 * anillo de foco por dentro— sin que el menú se dibuje la suya. Antes era un
 * botón propio que crecía, se corría y se pintaba de rosa al pasar por encima.
 *
 * El nombre se parte en dos líneas en vez de cortarse: no hay otro lugar donde
 * leerlo entero.
 */
const props = defineProps({
	app: {
		type: Object,
		required: true,
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
  <DropdownMenuItem :title="app.description" class="w-full" @select="openApp">
    <ThemeIcon :name="app.icon" :size="32" alt="" />
    <span class="min-w-0 flex-1 break-words text-left">{{ app.name }}</span>
  </DropdownMenuItem>
</template>
