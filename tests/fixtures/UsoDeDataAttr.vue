<script setup lang="ts">
/**
 * Un `data-*` cualquiera sobre un elemento.
 *
 * Con `strictTemplates`, `vue-tsc` comprueba que cada atributo exista en el
 * tipo del elemento, y los `data-*` no están: los permite la declaración de
 * `src/tipos-de-plantilla.d.ts`. Este archivo es la prueba de que esa
 * declaración hace algo — se compila con ella y sin ella.
 */
</script>

<template>
	<div data-marca-de-prueba="1">hola</div>
</template>
