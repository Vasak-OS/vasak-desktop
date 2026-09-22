<script lang="ts" setup>
import { ThemeIcon } from '@vasakgroup/vue-libvasak';

// A plain <button>, not an <a href="#">.
//
// The router uses hash history, so clicking an href="#" anchor cleared
// location.hash, navigated to a path with no matching route, and left the menu
// window rendering nothing.
defineProps({
	title: String,
	/** El nombre del icono en el tema del escritorio, no una ruta. */
	icono: String,
});

// El clic va declarado y cableado a mano. Venía llegando por caída de atributos
// —el `@click` del padre aterrizaba sobre el `<button>`—, que funciona pero no
// se puede comprobar. Al declararlo se corta esa caída, así que si no se
// reemite acá el botón deja de responder.
const emit = defineEmits<{ click: [MouseEvent] }>();
</script>

<template>
  <button type="button" :title="title" class="theme-transition" @click="emit('click', $event)">
    <ThemeIcon :name="icono ?? ''" :size="32" :alt="title" />
  </button>
</template>
