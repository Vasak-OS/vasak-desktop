<script lang="ts" setup>
import { computed } from 'vue';
import AppMenuButton from '@/components/buttons/AppMenuButton.vue';

const props = defineProps({
	apps: {
		type: Array,
		required: true
	},
	filter: {
		type: String,
		required: true
	}
});

const appsFiltred = computed((): Array<any> => {
	return props.apps.filter(
		(app: any) =>
			app.name.toLowerCase().includes(props.filter) || app.description.toLowerCase().includes(props.filter)
	);
});
</script>

<template>
  <transition-group
    tag="div"
    name="search-filter"
    appear
    class="flex flex-wrap gap-1 p-0.5"
  >
    <AppMenuButton 
      v-for="app in appsFiltred" 
      :key="app.name" 
      :app="app" 
      class="search-app-item"
    />
  </transition-group>
</template>

<style scoped>
.search-results-container {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 0.75rem;
  padding: 1rem;
}

.search-app-item {
  transition: all 0.3s ease;
}

/* Animaciones para la búsqueda */
.search-filter-enter-active {
  transition: all 0.5s ease-out;
}

.search-filter-leave-active {
  transition: all 0.4s ease-in;
}

.search-filter-enter-from {
  opacity: 0;
  transform: scale(0.8) translateY(20px);
}

.search-filter-leave-to {
  opacity: 0;
  transform: scale(0.9) translateX(-10px);
}

.search-filter-move {
  transition: transform 0.3s ease;
}

/* Efecto hover mejorado para elementos de búsqueda */
.search-app-item:hover {
  transform: translateY(-2px) scale(1.02);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}
</style>
