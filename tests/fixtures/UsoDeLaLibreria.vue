<!--
  Uso correcto de la librería, para que el test lo compruebe contra los tipos
  que ella publica. Si esto deja de pasar el chequeo, los tipos cambiaron.
-->
<script setup lang="ts">
import { WindowFrame } from '@vasakgroup/vue-libvasak';
</script>

<template>
  <WindowFrame :title="'Escritorio'" :hide-bar="false">
    <template #identidad><span>vsk</span></template>
    <div>contenido</div>
  </WindowFrame>
</template>
