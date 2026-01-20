<script lang="ts" setup>
import { onMounted, Ref, ref } from 'vue';
import { getIconSource } from '@vasakgroup/plugin-vicons';

const props = defineProps<{
  to: string;
  icon: string;
}>();
const iconSrc: Ref<string> = ref('');

onMounted(async () => {
	iconSrc.value = await getIconSource(props.icon);
});
</script>
<template>
  <li class="w-14 h-14 my-4">
  <RouterLink
    :to="to"
    class="relative block w-14 h-14 p-2 rounded-vsk transition-all duration-200 hover:shadow-lg hover:bg-vsk-primary/20"
    active-class="bg-vsk-primary/20 shadow-lg scale-105"
    exact-active-class="bg-vsk-primary/30 shadow-xl scale-110 ring-2 ring-vsk-primary/50"
  >

    <!-- Icono con efecto de escala -->
    <img
      :src="iconSrc"
      :alt="`Icon for ${to}`"
      class="w-full h-full transition-transform duration-200 hover:scale-110"
      :class="{ 'scale-110': $route.path === to }"
    />
  </RouterLink>
  </li>
</template>