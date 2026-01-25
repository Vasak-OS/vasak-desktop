<template>
  <div
    class="p-1 rounded-vsk relative hover:bg-vsk-primary/30 group transition-all duration-300"
    :class="customClass"
    :title="tooltip"
    @click="handleClick"
    @mouseenter="showTooltip = true"
    @mouseleave="showTooltip = false"
  >
    <img
      :src="icon"
      :alt="alt"
      class="m-auto h-5.5 w-auto transition-all duration-300"
      :class="iconClass"
    />
    
    <!-- Badge/Counter -->
    <div
      v-if="badge !== null && badge > 0"
      class="absolute bottom-1 right-1 bg-vsk-primary text-white text-xs rounded-full w-4 h-4 flex items-center justify-center font-bold animate-bounce"
    >
      {{ badge }}
    </div>

    <!-- Tooltip personalizado -->
    <div 
      v-if="showCustomTooltip && customTooltipText"
      class="absolute top-1 left-1/2 transform -translate-x-1/2 text-xs font-semibold p-1 rounded-vsk transition-all duration-300 pointer-events-none background"
      :class="[
        tooltipClass,
        {
          'opacity-0 -translate-y-2': !showTooltip,
          'opacity-100 translate-y-0': showTooltip
        }
      ]"
    >
      {{ customTooltipText }}
    </div>

    <!-- Slot para contenido adicional personalizado -->
    <slot></slot>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';

interface Props {
	icon: string;
	alt?: string;
	tooltip?: string;
	badge?: number | null;
	iconClass?: string | Record<string, boolean>;
	customClass?: string | Record<string, boolean>;
	tooltipClass?: string | Record<string, boolean>;
	showCustomTooltip?: boolean;
	customTooltipText?: string;
}

withDefaults(defineProps<Props>(), {
	alt: '',
	tooltip: '',
	badge: null,
	iconClass: () => ({}),
	customClass: '',
	tooltipClass: '',
	showCustomTooltip: false,
	customTooltipText: '',
});

const emit = defineEmits<{
	click: [];
}>();

const showTooltip = ref(false);

const handleClick = () => {
	emit('click');
};
</script>
