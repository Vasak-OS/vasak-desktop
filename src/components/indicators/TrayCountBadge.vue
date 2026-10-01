<script setup lang="ts">
/**
 * El contador sobre un icono de la bandeja. **Provisorio.**
 *
 * Es la insignia de `TrayIconButton` de vue-libvasak 2.0.0 —las mismas clases,
 * la píldora `rounded-corner-full` en `primary` con el texto encima—, que es la
 * que ya llevan las ventanas del panel: así el contador de un elemento de la
 * bandeja y el de una ventana se ven iguales. Va aparte porque los elementos de
 * la bandeja no son un `TrayIconButton` (dibujan el mapa de bits de otra
 * aplicación y distinguen los botones del ratón).
 *
 * Se va cuando el escritorio suba a vue-libvasak 2.1.0, que trae `Badge` y hace
 * que `TrayIconButton` la use por dentro: entonces esto pasa a ser
 * `<Badge tone="accent" variant="solid">` de la librería.
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
