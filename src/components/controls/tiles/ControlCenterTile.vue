<script setup lang="ts">
/**
 * Un mosaico del estado B del centro de control (vasak-desktop#175).
 *
 * Es una `ListRow` de la librería, sin estilo propio: icono del tema, el nombre,
 * el estado en la segunda línea y, si abre un detalle, la flecha › del tema
 * (`go-next`). La librería todavía no tiene un mosaico de ajuste rápido con
 * estado encendido/apagado a la Once UI; cuando lo tenga, este componente pasa
 * a usarlo y los mosaicos no cambian. Mientras tanto el estado se **dice** en
 * la segunda línea y en `aria-pressed`, no se pinta con colores escritos acá.
 *
 * Un mosaico hace **una** cosa al tocarlo: alterna (`pressed` con valor), abre
 * su detalle dentro del bloque (`hasDetail`) o abre otra ventana. Nada de
 * botones adentro de otro botón.
 */
import { ListRow, ThemeIcon } from '@vasakgroup/vue-libvasak';

withDefaults(
	defineProps<{
		/** El nombre del icono en el tema del escritorio (variante simbólica). */
		icon: string;
		title: string;
		/** El estado, ya traducido: «Conectado a Casa», «Apagado». */
		description?: string;
		/** Si al tocarlo se abre su detalle dentro del bloque. */
		hasDetail?: boolean;
		/** Encendido o apagado, cuando el mosaico alterna algo; `null` si no. */
		pressed?: boolean | null;
		disabled?: boolean;
	}>(),
	{ description: undefined, hasDetail: false, pressed: null, disabled: false }
);

const emit = defineEmits<{ click: [] }>();
</script>

<template>
  <ListRow
    role="button"
    :icon="icon"
    icon-type="symbol"
    :title="title"
    :description="description"
    :disabled="disabled"
    :aria-pressed="pressed ?? undefined"
    :aria-haspopup="hasDetail ? 'true' : undefined"
    data-tile
    @click="emit('click')"
  >
    <template v-if="hasDetail" #trailing>
      <ThemeIcon name="go-next" type="symbol" :size="16" alt="" />
    </template>
  </ListRow>
</template>
