<script setup lang="ts">
/**
 * El contador sobre un icono de la bandeja.
 *
 * Es la insignia de `TrayIconButton` de vue-libvasak —las mismas clases, la
 * píldora `rounded-corner-full` en `primary` con el texto encima—, que es la
 * que llevan las ventanas del panel: así el contador de un elemento de la
 * bandeja y el de una ventana se ven iguales. Va aparte porque los elementos de
 * la bandeja no son un `TrayIconButton` (dibujan el mapa de bits de otra
 * aplicación y distinguen los botones del ratón).
 *
 * **No es `Badge` de la librería, y no por descuido.** Probado en el banco con
 * vue-libvasak 2.2.0 (`tone="accent" variant="solid"`): `Badge` lleva
 * `max-w-full` y `break-words`, así que puesto en absoluto sobre un icono de
 * 28 px se queda con el ancho del icono y parte «99+» en una columna de tres
 * renglones que tapa al vecino. Es lo que pasó con la copia de #146. Se va
 * cuando `Badge` tenga una forma de contador que no se parta (sin `max-w-full`
 * ni `break-words`, con `whitespace-nowrap`); el arreglo es de la librería.
 */
defineProps<{
	label: string;
}>();
</script>

<template>
  <span
    class="flex h-4 min-w-4 items-center justify-center rounded-corner-full bg-primary px-1 font-semibold whitespace-nowrap text-label-xs text-tx-on-primary"
  >
    {{ label }}
  </span>
</template>
