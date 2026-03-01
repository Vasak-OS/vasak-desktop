<script setup lang="ts">
interface Props {
	label: string;
	disabled?: boolean;
	variant?: 'primary' | 'secondary' | 'danger';
	loading?: boolean;
	customClass?: string | Record<string, boolean>;
	size?: 'sm' | 'md' | 'lg';
	fullWidth?: boolean;
	iconSrc?: string;
	iconAlt?: string;
	iconRight?: boolean;
	type?: 'button' | 'submit' | 'reset';
	stopPropagation?: boolean;
	preventDefault?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
	disabled: false,
	variant: 'primary',
	loading: false,
	customClass: () => ({}),
	size: 'md',
	fullWidth: false,
	iconSrc: '',
	iconAlt: '',
	iconRight: false,
	type: 'button',
	stopPropagation: false,
	preventDefault: false,
});

const emit = defineEmits<{
	click: [];
}>();

const variantClasses: Record<string, string> = {
	primary:
		'bg-primary dark:bg-primary-dark text-tx-on-primary dark:text-tx-on-primary-dark hover:bg-primary/90 dark:hover:bg-primary-dark/90',
	secondary:
		'bg-secondary dark:bg-secondary-dark text-tx-on-primary dark:text-tx-on-primary-dark hover:bg-secondary/80 dark:hover:bg-secondary-dark/80',
	danger:
		'bg-status-error dark:bg-status-error-dark text-tx-on-primary dark:text-tx-on-primary-dark hover:bg-status-error/90 dark:hover:bg-status-error-dark/90',
};

const sizeClasses: Record<'sm' | 'md' | 'lg', string> = {
	sm: 'px-2 py-1 text-xs',
	md: 'px-3 py-1 text-sm',
	lg: 'px-4 py-2 text-base',
};

const handleClick = (event: Event) => {
	if (props.stopPropagation) event.stopPropagation();
	if (props.preventDefault) event.preventDefault();
	if (!props.disabled && !props.loading) {
		emit('click');
	}
};
</script>

<template>
  <button
    :type="props.type"
    @click="handleClick"
    class="rounded-corner transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
    :class="[
      variantClasses[props.variant],
      sizeClasses[props.size],
      props.fullWidth ? 'w-full' : '',
      props.iconSrc && !props.label ? 'px-2 py-2' : '',
      customClass,
    ]"
    :disabled="props.disabled || props.loading"
  >
    <span v-if="loading" class="w-4 h-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
    <template v-if="props.iconSrc && !props.iconRight">
      <img :src="props.iconSrc" :alt="props.iconAlt || props.label" class="w-4 h-4" />
    </template>
    <span v-if="props.label">{{ props.label }}</span>
    <template v-if="props.iconSrc && props.iconRight">
      <img :src="props.iconSrc" :alt="props.iconAlt || props.label" class="w-4 h-4" />
    </template>
  </button>
</template>
