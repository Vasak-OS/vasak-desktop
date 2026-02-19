<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
import { WindowFrame } from '@vasakgroup/vue-libvasak';

defineProps<{
	containerClass?: string;
}>();
</script>

<template>
  <WindowFrame>
    <div
      class="h-[calc(100vh-32px)] rounded-b-window background"
      :class="containerClass"
    >
      <slot />
    </div>
  </WindowFrame>
</template>
