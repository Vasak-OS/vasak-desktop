<script setup lang="ts">
interface Props {
	label: string;
	htmlFor?: string;
	customClass?: string | Record<string, boolean>;
	labelClass?: string | Record<string, boolean>;
}

withDefaults(defineProps<Props>(), {
	htmlFor: '',
	customClass: () => ({}),
	labelClass: () => ({}),
});
</script>

<template>
  <div class="flex flex-col gap-2" :class="customClass">
    <label v-if="label" :for="htmlFor" class="text-sm font-medium text-vsk-primary" :class="labelClass">
      {{ label }}
    </label>
    <slot />
  </div>
</template>

<style scoped>
/* Ningún estilo adicional requerido */
</style>
