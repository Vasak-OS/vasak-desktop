<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { onMounted, type Ref, ref } from 'vue';

const emit = defineEmits(['update:categorySelected']);

const props = defineProps<{
	category: any;
	image: string;
	description: string;
	categorySelected: string;
}>();

const appIcon: Ref<string> = ref(props.image);

const setCategory = (category: string) => {
	emit('update:categorySelected', category);
};

const getAppIcon = async () => {
	appIcon.value = await getIconSource(props.image);
};

onMounted(() => {
	getAppIcon();
});
</script>

<template>
  <button
    class="p-2 rounded-vsk hover:scale-120"
    @click="setCategory(category)"
    :class="{
      'bg-vsk-primary/50 selected-category': category === categorySelected,
      'bg-transparent': category !== categorySelected,
    }"
  >
    <img :src="appIcon" :title="description" :alt="category" class="h-12" />
  </button>
</template>

<style scoped>
@keyframes pulse {
  0% {
    transform: scale(1);
    box-shadow: 0 0 0 0 rgba(255, 255, 255, 0.2);
  }

  50% {
    transform: scale(1.02);
    box-shadow: 0 0 0 10px rgba(255, 255, 255, 0);
  }

  100% {
    transform: scale(1);
    box-shadow: 0 0 0 0 rgba(255, 255, 255, 0);
  }
}

@keyframes glow {
  0% {
    border-color: rgba(127, 127, 127, 0.1);
  }
  50% {
    border-color: rgba(127, 127, 127, 0.3);
  }
  100% {
    border-color: rgba(127, 127, 127, 0.1);
  }
}

.selected-category {
  animation: pulse 2s infinite ease-in-out;
  border: 1px solid rgba(127, 127, 127, 0.1);
  position: relative;
}

.selected-category::after {
  content: "";
  position: absolute;
  inset: -1px;
  border-radius: inherit;
  background: linear-gradient(
    45deg,
    rgba(127, 127, 127, 0) 0%,
    rgba(127, 127, 127, 0.1) 50%,
    rgba(127, 127, 127, 0) 100%
  );
  animation: shine 3s infinite linear;
}

@keyframes shine {
  0% {
    background-position: -200% 0;
  }
  100% {
    background-position: 200% 0;
  }
}

.category-pill {
  backdrop-filter: blur(8px);
}

.category-pill:hover {
  transform: translateX(4px);
}
</style>
