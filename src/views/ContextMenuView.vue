<script setup lang="ts">
import { useContextMenuWindow } from '@vasakgroup/plugin-vsk-contextual-menu';

/**
 * La ventana donde el plugin dibuja el menú cuando no entra en la que lo pide.
 *
 * El panel mide unos treinta píxeles de alto: un menú dibujado adentro quedaría
 * recortado a la primera línea, así que ahí se usa el modo ventana y el plugin
 * carga esta ruta. No hay nada que escribir acá: el plugin se encarga de todo.
 */
useContextMenuWindow();
</script>

<template>
	<div />
</template>
