<template>
  <button
    type="button"
    @click="handleClick"
    :disabled="disabled"
    :class="[
      'relative inline-flex items-center rounded-full transition-colors',
      size === 'small' ? 'h-6 w-11' : 'h-7 w-12',
      isOn ? activeClass : inactiveClass,
      disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer',
      customClass
    ]"
  >
    <span
      :class="[
        'inline-block transform rounded-full bg-white shadow transition-transform',
        size === 'small' ? 'h-4 w-4' : 'h-6 w-6',
        isOn ? (size === 'small' ? 'translate-x-6' : 'translate-x-5') : 'translate-x-1'
      ]"
    ></span>
  </button>
</template>

<script setup lang="ts">
interface Props {
	isOn: boolean;
	disabled?: boolean;
	size?: 'small' | 'medium';
	activeClass?: string;
	inactiveClass?: string;
	customClass?: string;
}

const props = withDefaults(defineProps<Props>(), {
	disabled: false,
	size: 'small',
	activeClass: 'bg-vsk-primary',
	inactiveClass: 'background',
	customClass: '',
});

const emit = defineEmits<{
	toggle: [value: boolean];
}>();

const handleClick = () => {
	emit('toggle', !props.isOn);
};
</script>
