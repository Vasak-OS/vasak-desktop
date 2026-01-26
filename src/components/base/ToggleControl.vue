<template>
  <button
    @click="handleClick"
    class="p-2 rounded-vsk background hover:opacity-50 transition-all duration-300 h-17.5 w-17.5 group relative overflow-hidden hover:scale-105 hover:shadow-lg active:scale-95"
    :class="{
      'animate-pulse': isLoading,
      'ring-2 ring-vsk-primary/50': isActive,
      'opacity-60': !isActive,
      ...customClass
    }"
    :disabled="isLoading"
  >
    <img
      :src="icon"
      :alt="alt"
      :title="tooltip"
      class="m-auto w-12.5 h-12.5 transition-all duration-300 group-hover:scale-110 relative z-10"
      :class="{
        'animate-spin': isLoading,
        'filter brightness-75': !isActive,
        'drop-shadow-lg': isActive,
        ...iconClass
      }"
    />
  </button>
</template>

<script setup lang="ts">
interface Props {
	icon: string;
	alt?: string;
	tooltip?: string;
	isActive?: boolean;
	isLoading?: boolean;
	iconClass?: Record<string, boolean>;
	customClass?: Record<string, boolean>;
}

withDefaults(defineProps<Props>(), {
	alt: '',
	tooltip: '',
	isActive: false,
	isLoading: false,
	iconClass: () => ({}),
	customClass: () => ({}),
});

const emit = defineEmits<{
	click: [];
}>();

const handleClick = () => {
	emit('click');
};
</script>
