<script lang="ts" setup>
import AppMenuCard from '@/components/cards/AppMenuCard.vue';

defineProps({
	apps: {
		type: Array<any>,
		required: true,
	},
});
</script>

<template>
  <div class="flex flex-wrap flex-col items-center" >
    <AppMenuCard v-for="app in apps" :key="app.name" :app="app" />
  </div>
</template>
