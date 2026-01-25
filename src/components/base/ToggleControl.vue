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
    <!-- Background glow effect -->
    <div
      class="absolute inset-0 rounded-vsk bg-lineal-to-br transition-opacity duration-300"
      :class="[glowClass, { 'opacity-0': !isActive, 'group-hover:opacity-100': isActive }]"
    ></div>

    <!-- Status indicator -->
    <div
      v-if="showStatusIndicator"
      class="absolute top-1 right-1 w-3 h-3 rounded-full transition-all duration-300"
      :class="statusIndicatorClass"
    ></div>

    <!-- Badge/Counter -->
    <div
      v-if="badge !== null && badge > 0"
      class="absolute bottom-1 right-1 bg-vsk-primary text-white text-xs rounded-full w-4 h-4 flex items-center justify-center font-bold animate-bounce"
    >
      {{ badge }}
    </div>

    <!-- Waves animation when enabled -->
    <div
      v-if="showWaves && isActive"
      class="absolute inset-0 flex items-center justify-center"
    >
      <div
        class="absolute w-8 h-8 border-2 border-vsk-primary/30 rounded-full animate-ping"
      ></div>
      <div
        class="absolute w-12 h-12 border-2 border-vsk-primary/20 rounded-full animate-ping"
        style="animation-delay: 0.5s"
      ></div>
    </div>

    <!-- Signal strength indicator (para network) -->
    <div
      v-if="showSignalStrength && signalStrength !== null"
      class="absolute bottom-1 left-1 flex space-x-0.5"
    >
      <div
        v-for="i in 4"
        :key="i"
        class="w-1 bg-vsk-primary rounded-full transition-all duration-300"
        :class="{
          'opacity-100': i <= Math.ceil(signalStrength / 25),
          'opacity-30': i > Math.ceil(signalStrength / 25),
        }"
        :style="{ height: `${4 + i * 2}px` }"
      ></div>
    </div>

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
	badge?: number | null;
	showStatusIndicator?: boolean;
	statusIndicatorClass?: string | Record<string, boolean>;
	showWaves?: boolean;
	showSignalStrength?: boolean;
	signalStrength?: number | null;
	glowClass?: string;
	iconClass?: Record<string, boolean>;
	customClass?: Record<string, boolean>;
}

withDefaults(defineProps<Props>(), {
	alt: '',
	tooltip: '',
	isActive: false,
	isLoading: false,
	badge: null,
	showStatusIndicator: true,
	statusIndicatorClass: () => ({ 'bg-blue-400': true }),
	showWaves: false,
	showSignalStrength: false,
	signalStrength: null,
	glowClass: 'from-blue-500/20 to-indigo-500/20',
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
