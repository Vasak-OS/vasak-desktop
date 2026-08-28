<template>
  <div
    class="theme-transition bg-ui-bg/80 rounded-corner flex border border-ui-border flex-row items-center gap-2 justify-between w-full h-auto p-4 transition-all duration-200 hover:bg-ui-surface/80"
  >
    <button
      v-if="showButton"
      @click="handleButtonClick"
      :title="buttonLabel ?? label"
      :aria-label="buttonLabel ?? label"
      class="w-8 h-8 flex items-center justify-center rounded-corner transition-all duration-200 hover:bg-ui-surface/80 hover:scale-110 active:scale-95"
    >
      <img :src="icon" alt="" class="w-6 h-6 transition-all duration-200" :class="iconClass" />
    </button>
    
    <div
      v-else
      class="w-8 h-8 flex items-center justify-center"
    >
      <img :src="icon" alt="" class="w-6 h-6 transition-all duration-200" />
    </div>

    <input
      type="range"
      :min="min"
      :max="max"
      :value="modelValue"
      @input="handleInput"
      :aria-label="label"
      :aria-valuetext="`${percentage}%`"
      class="flex-1 transition-all duration-200 hover:scale-105 accent-primary hover:accent-secondary"
    />
    
    <span
      class="w-12 text-right transition-all duration-200 font-medium"
      :class="percentageClass"
    >
      {{ percentage }}%
    </span>
  </div>
</template>

<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { computed } from 'vue';

interface Props {
	icon: string;
	/**
	 * Nombre de lo que regula el deslizador, ya traducido. Obligatorio: el
	 * `input` no tiene `<label>` asociado ni texto propio.
	 */
	label: string;
	/** Nombre de la acción del botón, si difiere del deslizador. */
	buttonLabel?: string;
	modelValue: number;
	min?: number;
	max?: number;
	showButton?: boolean;
	iconClass?: string | Record<string, boolean>;
	getPercentageClass?: (percentage: number) => string;
}

const props = withDefaults(defineProps<Props>(), {
	min: 0,
	max: 100,
	showButton: false,
	iconClass: () => ({}),
	getPercentageClass: () => '',
});

const emit = defineEmits<{
	'update:modelValue': [value: number];
	buttonClick: [];
}>();

const percentage = computed(() => {
	if (props.max <= props.min) return 0;
	const range = props.max - props.min;
	const value = props.modelValue - props.min;
	return Math.round((value / range) * 100);
});

const percentageClass = computed(() => {
	if (props.getPercentageClass) {
		return props.getPercentageClass(percentage.value);
	}
	return '';
});

const handleInput = (event: Event) => {
	const target = event.target as HTMLInputElement;
	emit('update:modelValue', Number(target.value));
};

const handleButtonClick = () => {
	emit('buttonClick');
};
</script>
