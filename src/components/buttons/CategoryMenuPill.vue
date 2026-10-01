<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { DropdownMenuItem, ThemeIcon } from '@vasakgroup/vue-libvasak';

/**
 * Una categoría del menú.
 *
 * Es el ítem de menú de la librería, del tamaño de su celda y con el icono de
 * siempre (40, y 56 el de «todas»): el velo neutro `ui-hover` al pasar en lugar
 * de crecer un 10 %, el anillo de foco por dentro, y la elegida con el velo del
 * acento (`ui-selected-accent`, decisión 4 de vue-libvasak#74) en lugar del
 * relleno entero del primario con borde del secundario. No es un
 * `ActionButton` porque ése dibuja el icono a 16, el de los controles, y acá el
 * icono es todo el contenido: a 16 en una celda de cien píxeles no se lee.
 */
const emit = defineEmits(['update:categorySelected']);

const props = defineProps<{
	category: any;
	image: string;
	/** Nombre de la categoría, ya traducido: es lo único que nombra al botón. */
	label: string;
	categorySelected: string;
	large?: boolean;
}>();

const setCategory = () => {
	emit('update:categorySelected', props.category);
};
</script>

<template>
  <DropdownMenuItem
    :aria-label="label"
    :aria-current="category === categorySelected || undefined"
    class="h-full w-full justify-center"
    :class="category === categorySelected ? 'bg-ui-selected-accent' : ''"
    @select="setCategory">
    <span class="flex h-full w-full items-center justify-center" :title="label">
      <ThemeIcon :name="image" :size="large ? 56 : 40" alt="" />
    </span>
  </DropdownMenuItem>
</template>
