<script setup lang="ts">
interface Props {
	title: string;
	icon?: string;
	customClass?: string | Record<string, boolean>;
}

withDefaults(defineProps<Props>(), {
	icon: '',
	customClass: () => ({}),
});
</script>

<template>
  <div
    class="flex flex-col gap-4 p-4 background rounded-vsk"
    :class="customClass"
  >
    <h3 class="text-base font-semibold m-0 text-vsk-primary">
      {{ icon ? `${icon} ${title}` : title }}
    </h3>
    <slot />
  </div>
</template>

<style scoped>
/* Ningún estilo adicional requerido */
</style>
