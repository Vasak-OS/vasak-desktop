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
    class="flex flex-col gap-4 p-4 bg-ui-bg/80 rounded-corner"
    :class="customClass"
  >
    <h3 class="text-base font-semibold m-0 text-primary">
      {{ icon ? `${icon} ${title}` : title }}
    </h3>
    <slot />
  </div>
</template>
