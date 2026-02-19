<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import type { Component } from 'vue';
import ConfigAppLayout from '@/layouts/ConfigAppLayout.vue';

type ComponentProps = Record<string, unknown>;

const props = defineProps<{
	component: Component;
	componentProps?: ComponentProps;
}>();
</script>

<template>
  <ConfigAppLayout>
    <component :is="props.component" v-bind="props.componentProps" />
  </ConfigAppLayout>
</template>
